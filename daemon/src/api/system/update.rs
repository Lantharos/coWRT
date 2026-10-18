use std::time::Duration;

use axum::Json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::http::{ApiError, ApiResult};
use crate::outbound::Request;

const RELEASES: &str = "https://api.github.com/repos/Lantharos/coWRT/releases/latest";
const PACKAGE: &str = "/tmp/cowrt-update.apk";
const LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Deserialize)]
struct Asset {
	name: String,
	browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
	tag_name: String,
	body: Option<String>,
	assets: Vec<Asset>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
	current: &'static str,
	latest: String,
	notes: Option<String>,
	available: bool,
}

fn version(tag: &str) -> Vec<u32> {
	tag.trim_start_matches('v')
		.split('.')
		.map(|part| part.parse().unwrap_or(0))
		.collect()
}

async fn latest() -> Result<Release, ApiError> {
	Request::get(RELEASES)
		.header("Accept", "application/vnd.github+json")
		.json(None::<()>)
		.await
		.map_err(|err| ApiError::conflict(format!("Couldn't reach GitHub: {err}")))
}

async fn architecture() -> Result<String, ApiError> {
	let output = Command::new("apk").arg("--print-arch").output().await?;
	Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub async fn check_update() -> ApiResult<UpdateInfo> {
	let release = latest().await?;
	let current = env!("CARGO_PKG_VERSION");
	Ok(Json(UpdateInfo {
		available: version(&release.tag_name) > version(current),
		current,
		latest: release.tag_name.trim_start_matches('v').to_owned(),
		notes: release.body,
	}))
}

fn asset<'a>(release: &'a Release, name: &str) -> Result<&'a Asset, ApiError> {
	release
		.assets
		.iter()
		.find(|a| a.name == name)
		.ok_or_else(|| ApiError::conflict(format!("This release has no {name}")))
}

pub async fn install_update() -> Result<(), ApiError> {
	let release = latest().await?;
	let latest = release.tag_name.trim_start_matches('v');
	if version(latest) <= version(env!("CARGO_PKG_VERSION")) {
		return Err(ApiError::conflict("You already have the newest version"));
	}
	let name = format!("cowrt-{latest}-{}.apk", architecture().await?);
	let package = asset(&release, &name)?;
	let sums = asset(&release, "SHA256SUMS")?;
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
