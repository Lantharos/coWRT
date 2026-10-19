use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::system::outbound::Request;
use crate::system::packages;

const VERSIONS: &str = "https://downloads.openwrt.org/.versions.json";
const FRESH_FOR: Duration = Duration::from_secs(600);
const KEEP: &str = "/lib/upgrade/keep.d/cowrt-upgrade";
const COWRT_FILES: [&str; 5] = [
	"/usr/sbin/cowrtd",
	"/usr/share/cowrt/",
	"/etc/init.d/cowrtd",
	"/etc/rc.d/S95cowrtd",
	"/lib/upgrade/keep.d/cowrt",
];

#[derive(Clone, Default, Serialize)]
#[serde(tag = "stage", rename_all = "lowercase")]
pub enum Progress {
	#[default]
	Idle,
	Preparing,
	Building,
	Verifying,
	Installing,
	Failed {
		reason: String,
	},
}

#[derive(Default)]
pub struct Upgrade {
	progress: Mutex<Progress>,
	latest: Mutex<Option<(Instant, String)>>,
}

#[derive(Deserialize)]
struct Versions {
	stable_version: String,
}

impl Upgrade {
	pub fn progress(&self) -> Progress {
		self.progress.lock().expect("upgrade lock poisoned").clone()
	}

	fn set(&self, progress: Progress) {
		*self.progress.lock().expect("upgrade lock poisoned") = progress;
	}

	pub fn begin(&self) -> bool {
		let mut progress = self.progress.lock().expect("upgrade lock poisoned");
		if matches!(*progress, Progress::Idle | Progress::Failed { .. }) {
			*progress = Progress::Preparing;
			return true;
		}
		false
	}

	pub async fn latest(&self) -> io::Result<String> {
		if let Some((at, version)) = &*self.latest.lock().expect("upgrade lock poisoned")
			&& at.elapsed() < FRESH_FOR
		{
			return Ok(version.clone());
		}
		let versions: Versions = Request::get(VERSIONS).json(None::<()>).await?;
		*self.latest.lock().expect("upgrade lock poisoned") = Some((Instant::now(), versions.stable_version.clone()));
		Ok(versions.stable_version)
	}

	pub async fn run(&self, version: &str) {
		if let Err(reason) = upgrade(self, version).await {
			let _ = tokio::fs::remove_file(KEEP).await;
			self.set(Progress::Failed { reason });
		}
	}
}

fn release_key(version: &str) -> Option<(Vec<u32>, bool)> {
	let (numbers, suffix) = version.split_once('-').map_or((version, None), |(n, s)| (n, Some(s)));
	let numbers = numbers
		.split('.')
		.map(|part| part.parse().ok())
		.collect::<Option<Vec<u32>>>()?;
	Some((numbers, suffix.is_none()))
}

pub fn newer(candidate: &str, current: &str) -> bool {
	match (release_key(candidate), release_key(current)) {
		(Some(candidate), Some(current)) => candidate > current,
		_ => false,
	}
}

async fn command(program: &str, args: &[&str]) -> Result<String, String> {
	let output = Command::new(program)
		.args(args)
		.output()
		.await
		.map_err(|err| format!("Couldn't run {program}: {err}"))?;
	if output.status.success() {
		return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
	}
	let text = format!(
		"{}{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
	let reason = lines
		.iter()
		.rev()
		.find_map(|line| line.strip_prefix("ERROR:"))
		.or(lines.last().copied())
		.unwrap_or("The upgrade tool stopped without saying why");
	Err(reason.trim().to_owned())
}

async fn dependencies() -> Result<Vec<String>, String> {
	let listed = command("apk", &["info", "-R", "cowrt"]).await?;
	let wanted: Vec<&str> = listed
		.lines()
		.skip(1)
		.map(str::trim)
		.filter(|name| !name.is_empty() && *name != "libc")
		.collect();
	let mut args = vec!["list", "--installed", "--providers"];
	args.extend(&wanted);
	let providers = command("apk", &args).await?;
	Ok(wanted
		.iter()
		.map(|name| {
			providers
				.lines()
				.find_map(|line| line.strip_prefix(&format!("<{name}> ")))
				.and_then(|rest| rest.split_whitespace().next())
				.map_or_else(|| (*name).to_owned(), package_name)
		})
		.collect())
}

fn package_name(versioned: &str) -> String {
	let version_start = versioned
		.char_indices()
		.find(|&(at, c)| c == '-' && versioned[at + 1..].starts_with(|c: char| c.is_ascii_digit()))
		.map_or(versioned.len(), |(at, _)| at);
	versioned[..version_start].to_owned()
}

async fn upgrade(state: &Upgrade, version: &str) -> Result<(), String> {
	packages::ensure(&["owut"]).await.map_err(|err| err.to_string())?;
	let mut args = vec!["-V".to_owned(), version.to_owned()];
	if packages::installed("cowrt").await {
		args.extend([
			"-r".to_owned(),
			"cowrt".to_owned(),
			"-a".to_owned(),
			dependencies().await?.join(","),
		]);
	}
	let args: Vec<&str> = args.iter().map(String::as_str).collect();
	tokio::fs::write(KEEP, COWRT_FILES.join("\n") + "\n")
		.await
		.map_err(|err| format!("Couldn't prepare to keep coWRT: {err}"))?;

	state.set(Progress::Building);
	command("owut", &[&["download"], args.as_slice()].concat()).await?;
	state.set(Progress::Verifying);
	command("owut", &[&["verify"], args.as_slice()].concat()).await?;
	state.set(Progress::Installing);
	command("owut", &["install"]).await?;
	Ok(())
}
