mod guard;
mod lists;
pub mod private_dns;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use lists::List;

use crate::devices::policy;
use crate::networks::Layout;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::state::{AppState, RUNTIME_DIR, unix_now};

const SECTION: &str = "protection";
const REFRESH: Duration = Duration::from_secs(24 * 3600);
const RETRY: Duration = Duration::from_secs(600);
const SUPERVISE: Duration = Duration::from_secs(5);
const ACTIVITY_SECTION: &str = "activity";

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Profile {
	Plain,
	Threats,
	Everything,
}

impl Profile {
	fn port(self) -> u16 {
		match self {
			Self::Plain => 5300,
			Self::Threats => 5301,
			Self::Everything => 5302,
		}
	}

	fn lists(self) -> &'static [List] {
		match self {
			Self::Plain => &[],
			Self::Threats => &[List::Threats],
			Self::Everything => &List::ALL,
		}
	}

	fn file(self, extension: &str) -> PathBuf {
		dns_dir().join(format!("{}.{extension}", self.port()))
	}
}

pub fn dns_dir() -> PathBuf {
	PathBuf::from(RUNTIME_DIR).join("dns")
}

static RESOLVERS: LazyLock<Mutex<HashMap<Profile, Child>>> = LazyLock::new(Mutex::default);
static UPDATED: LazyLock<std::sync::Mutex<Option<u64>>> = LazyLock::new(Default::default);

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub ads: Vec<String>,
	pub threats: Vec<String>,
	pub allow: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	#[serde(flatten)]
	pub settings: Settings,
	pub updated: Option<u64>,
	pub private_dns: private_dns::Settings,
	pub dns_fallback: bool,
}

pub async fn load(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", SECTION).await?.into_iter().next();
	Ok(section
		.map(|s| Settings {
			ads: s.list("ads"),
			threats: s.list("threats"),
			allow: s.list("allow"),
		})
		.unwrap_or_default())
}

pub async fn view(ubus: &Ubus) -> Result<View, ubus::Error> {
	let settings = load(ubus).await?;
	let private_dns = private_dns::load(ubus).await?;
	let updated = *UPDATED.lock().expect("protection timestamp");
	Ok(View {
		settings,
		updated,
		private_dns,
		dns_fallback: guard::fallen_back(),
	})
}

pub async fn store(ubus: &Ubus, settings: &Settings) -> Result<(), ubus::Error> {
	uci::upsert(ubus, "cowrt", SECTION, SECTION, json!({})).await?;
	uci::unset(ubus, "cowrt", SECTION, &["ads", "threats", "allow"]).await?;
	for (option, list) in [
		("ads", &settings.ads),
		("threats", &settings.threats),
		("allow", &settings.allow),
	] {
		if !list.is_empty() {
			uci::set(ubus, "cowrt", SECTION, json!({ option: list })).await?;
		}
	}
	Ok(())
}

pub async fn activity_enabled(ubus: &Ubus) -> Result<bool, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", ACTIVITY_SECTION).await?.into_iter().next();
	Ok(section.is_none_or(|s| s.get("enabled") != Some("0")))
}

pub async fn set_activity(ubus: &Ubus, enabled: bool) -> Result<(), ubus::Error> {
	let values = json!({ "enabled": if enabled { "1" } else { "0" } });
	uci::upsert(ubus, "cowrt", ACTIVITY_SECTION, ACTIVITY_SECTION, values).await
}

fn resolver_for(settings: &Settings, network: &str, logging: bool) -> Option<Profile> {
	profile_of(settings, network).or(logging.then_some(Profile::Plain))
}

fn profile_of(settings: &Settings, network: &str) -> Option<Profile> {
	if settings.ads.iter().any(|n| n == network) {
		Some(Profile::Everything)
	} else if settings.threats.iter().any(|n| n == network) {
		Some(Profile::Threats)
	} else {
		None
	}
}

pub async fn redirects(ubus: &Ubus) -> Result<Vec<(String, u16)>, ubus::Error> {
	let settings = load(ubus).await?;
	let logging = activity_enabled(ubus).await?;
	let layout = Layout::load(ubus).await?;
	let bridge = layout.bridge_name();
	Ok(layout
		.managed()
		.filter_map(|network| {
			let profile = resolver_for(&settings, &network.name, logging)?;
			Some((format!("{bridge}.{}", layout.vlan_of(&network.name)?), profile.port()))
		})
		.collect())
}

async fn start(profile: Profile, logging: bool) -> std::io::Result<Child> {
	let mut command = Command::new("dnsmasq");
	if logging {
		command.args([
			"--log-queries=extra".to_owned(),
			format!("--log-facility={}", profile.file("log").display()),
			"--log-async=50".to_owned(),
		]);
	}
	command
		.arg("--keep-in-foreground")
		.arg(format!("--conf-file={}", profile.file("conf").display()))
		.arg(format!("--port={}", profile.port()))
		.args([
			"--no-resolv",
			"--no-hosts",
			"--server=127.0.0.1#53",
			"--cache-size=4096",
			"--pid-file=",
			"--local-service",
		])
		.spawn()
}

pub async fn reopen_logs() {
	for child in RESOLVERS.lock().await.values() {
		if let Some(pid) = child.id() {
			unsafe { libc::kill(pid as i32, libc::SIGUSR2) };
		}
	}
}

async fn ensure_lists(profile: Profile) {
	for list in profile.lists() {
		if !lists::downloaded(*list) && lists::download(*list).await.is_ok() {
			*UPDATED.lock().expect("protection timestamp") = Some(unix_now());
		}
	}
}

pub async fn apply(ubus: &Ubus) -> std::io::Result<()> {
	let settings = load(ubus).await.map_err(std::io::Error::other)?;
	let logging = activity_enabled(ubus).await.map_err(std::io::Error::other)?;
	let managed: Vec<String> = uci::sections_of(ubus, "cowrt", "network")
		.await
		.map_err(std::io::Error::other)?
		.into_iter()
		.map(|s| s.name)
		.collect();
	let wanted: Vec<Profile> = [Profile::Everything, Profile::Threats, Profile::Plain]
		.into_iter()
		.filter(|profile| {
			managed
				.iter()
				.any(|network| resolver_for(&settings, network, logging) == Some(*profile))
		})
		.collect();

	tokio::fs::create_dir_all(dns_dir()).await?;
	let mut resolvers = RESOLVERS.lock().await;
	for (_, mut child) in resolvers.extract_if(|profile, _| !wanted.contains(profile)) {
		let _ = child.kill().await;
	}
	for profile in wanted {
		ensure_lists(profile).await;
		lists::compile(profile.lists(), &settings.allow, &profile.file("conf")).await?;
		if let Some(mut previous) = resolvers.remove(&profile) {
			let _ = previous.kill().await;
		}
		resolvers.insert(profile, start(profile, logging).await?);
	}
	drop(resolvers);
	policy::apply(ubus).await
}

async fn reap_strays() {
	let marker = format!("--conf-file={RUNTIME_DIR}/dns/");
	let Ok(mut entries) = tokio::fs::read_dir("/proc").await else {
		return;
	};
	while let Ok(Some(entry)) = entries.next_entry().await {
		let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
			continue;
		};
		let cmdline = tokio::fs::read(entry.path().join("cmdline")).await.unwrap_or_default();
		if String::from_utf8_lossy(&cmdline).contains(&marker) {
			unsafe { libc::kill(pid, libc::SIGTERM) };
		}
	}
	tokio::time::sleep(Duration::from_millis(300)).await;
}

async fn revive(ubus: &Ubus) {
	let mut resolvers = RESOLVERS.lock().await;
	let exited: Vec<Profile> = resolvers
		.iter_mut()
		.filter_map(|(profile, child)| child.try_wait().ok().flatten().map(|_| *profile))
		.collect();
	if exited.is_empty() {
		return;
	}
	let logging = activity_enabled(ubus).await.unwrap_or(true);
	for profile in exited {
		eprintln!(
			"cowrtd: the DNS resolver on port {} stopped, starting it again",
			profile.port()
		);
		match start(profile, logging).await {
			Ok(child) => {
				resolvers.insert(profile, child);
			}
			Err(err) => eprintln!("cowrtd: could not restart the DNS resolver on port {}: {err}", profile.port()),
		}
	}
}

pub fn spawn(state: AppState) {
	guard::spawn(state.clone());
	tokio::spawn(async move {
		reap_strays().await;
		if let Err(err) = apply(&state.ubus).await {
			eprintln!("cowrtd: could not start DNS protection: {err}");
		}
		loop {
			let complete = List::ALL.iter().all(|list| lists::downloaded(*list));
			let refresh_at = Instant::now() + if complete { REFRESH } else { RETRY };
			while Instant::now() < refresh_at {
				tokio::time::sleep(SUPERVISE).await;
				revive(&state.ubus).await;
			}
			let mut refreshed = false;
			for list in List::ALL {
				refreshed |= lists::download(list).await.is_ok();
			}
			if refreshed {
				*UPDATED.lock().expect("protection timestamp") = Some(unix_now());
				let _ = apply(&state.ubus).await;
			}
		}
	});
}
