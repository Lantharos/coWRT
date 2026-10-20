use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::http::ApiError;
use crate::system::outbound::Request;
use crate::system::upgrade::newer;

const RELEASES: &str = "https://api.github.com/repos/Lantharos/coWRT/releases/latest";
const PACKAGE: &str = "/tmp/cowrt-update.apk";
const LIMIT: u64 = 64 * 1024 * 1024;
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Asset {
	name: String,
	browser_download_url: String,
}

#[derive(Deserialize)]
pub struct Release {
	tag_name: String,
	pub body: Option<String>,
	assets: Vec<Asset>,
}

impl Release {
	pub fn version(&self) -> &str {
		self.tag_name.trim_start_matches('v')
	}

	pub fn is_newer(&self) -> bool {
		newer(self.version(), CURRENT)
	}

	fn asset(&self, name: &str) -> Result<&Asset, ApiError> {
		self.assets
			.iter()
			.find(|a| a.name == name)
			.ok_or_else(|| ApiError::conflict(format!("This release has no {name}")))
	}
}

pub async fn latest() -> Result<Release, ApiError> {
	Request::get(RELEASES)
		.header("Accept", "application/vnd.github+json")
		.json(None::<()>)
		.await
		.map_err(|err| ApiError::conflict(format!("Couldn't reach GitHub: {err}")))
}

async fn architecture() -> Result<String, ApiError> {
	Ok(tokio::fs::read_to_string("/etc/apk/arch").await?.trim().to_owned())
}

pub async fn install(release: &Release) -> Result<(), ApiError> {
	if !release.is_newer() {
		return Err(ApiError::conflict("You already have the newest version"));
	}
	let name = format!("cowrt-{}-{}.apk", release.version(), architecture().await?);
	let package = release.asset(&name)?;
	let sums = release.asset("SHA256SUMS")?;
	let download = |url: &str| Request::get(url).bytes(LIMIT);
	let (archive, sums) = tokio::try_join!(download(&package.browser_download_url), download(&sums.browser_download_url))
		.map_err(|err| ApiError::conflict(format!("The download failed: {err}")))?;
	let expected = String::from_utf8_lossy(&sums)
		.lines()
		.find_map(|line| {
			line.split_once("  ")
				.filter(|(_, file)| *file == name)
				.map(|(hash, _)| hash.to_owned())
		})
		.ok_or_else(|| ApiError::conflict("The release has no checksum for this router"))?;
	let actual: String = Sha256::digest(&archive).iter().map(|b| format!("{b:02x}")).collect();
	if actual != expected {
		return Err(ApiError::conflict(
			"The download didn't match its checksum, so it wasn't installed",
		));
	}
	tokio::fs::write(PACKAGE, archive).await?;
	tokio::spawn(async {
		tokio::time::sleep(Duration::from_secs(1)).await;
		let _ = Command::new("apk").args(["add", "--allow-untrusted", PACKAGE]).status().await;
	});
	Ok(())
}
