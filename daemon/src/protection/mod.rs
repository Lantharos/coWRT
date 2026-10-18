mod lists;
pub mod private_dns;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

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
pub const PORTS: [u16; 2] = [5301, 5302];

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Profile {
	Threats,
	Everything,
}

impl Profile {
	fn port(self) -> u16 {
		match self {
			Self::Threats => PORTS[0],
			Self::Everything => PORTS[1],
		}
	}

	fn lists(self) -> &'static [List] {
		match self {
			Self::Threats => &[List::Threats],
			Self::Everything => &List::ALL,
		}
	}

	fn conf(self) -> PathBuf {
		PathBuf::from(RUNTIME_DIR).join("dns").join(format!("{}.conf", self.port()))
	}
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
	let layout = Layout::load(ubus).await?;
	let bridge = layout.bridge_name();
	Ok(layout
		.managed()
		.filter_map(|network| {
			let profile = profile_of(&settings, &network.name)?;
			Some((format!("{bridge}.{}", layout.vlan_of(&network.name)?), profile.port()))
		})
		.collect())
}

async fn start(profile: Profile) -> std::io::Result<Child> {
	Command::new("dnsmasq")
		.arg("--keep-in-foreground")
		.arg(format!("--conf-file={}", profile.conf().display()))
		.arg(format!("--port={}", profile.port()))
		.args([
			"--no-resolv",
			"--no-hosts",
			"--server=127.0.0.1#53",
			"--cache-size=4096",
			"--pid-file=",
			"--local-service",
		])
		.kill_on_drop(true)
		.spawn()
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
	let managed: Vec<String> = uci::sections_of(ubus, "cowrt", "network")
		.await
		.map_err(std::io::Error::other)?
		.into_iter()
		.map(|s| s.name)
		.collect();
	let wanted: Vec<Profile> = [Profile::Everything, Profile::Threats]
		.into_iter()
		.filter(|profile| managed.iter().any(|network| profile_of(&settings, network) == Some(*profile)))
		.collect();

	let mut resolvers = RESOLVERS.lock().await;
	resolvers.retain(|profile, _| wanted.contains(profile));
	for profile in wanted {
		ensure_lists(profile).await;
		lists::compile(profile.lists(), &settings.allow, &profile.conf()).await?;
		resolvers.remove(&profile);
		resolvers.insert(profile, start(profile).await?);
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

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		reap_strays().await;
		if let Err(err) = apply(&state.ubus).await {
			eprintln!("cowrtd: could not start DNS protection: {err}");
		}
		loop {
			let complete = List::ALL.iter().all(|list| lists::downloaded(*list));
			tokio::time::sleep(if complete { REFRESH } else { RETRY }).await;
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
