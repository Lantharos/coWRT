use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::http::ApiError;
use crate::state::{AppState, RUNTIME_DIR};

const BACKUP: &str = "/tmp/cowrt/backup.tar.gz";
const RESTORE: &str = "/tmp/cowrt/restore.tar.gz";
const FIRMWARE: &str = "/tmp/firmware.bin";

async fn sysupgrade(args: &[&str]) -> Result<(), ApiError> {
	let output = Command::new("sysupgrade").args(args).output().await?;
	if output.status.success() {
		return Ok(());
	}
	let reason = String::from_utf8_lossy(&output.stderr);
	Err(ApiError::conflict(
		reason
			.lines()
			.last()
			.unwrap_or("The router refused the file")
			.trim()
			.to_owned(),
	))
}

async fn receive(body: Body, path: &str) -> Result<(), ApiError> {
	let _ = tokio::fs::create_dir_all(RUNTIME_DIR).await;
	let mut file = tokio::fs::File::create(path).await?;
	let mut stream = body.into_data_stream();
	while let Some(chunk) = stream.next().await {
		let chunk = chunk.map_err(|err| ApiError::bad_request(format!("The upload broke off: {err}")))?;
		file.write_all(&chunk).await?;
	}
	file.flush().await?;
	Ok(())
}

pub async fn backup(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
	state.monitor.save().await;
	sysupgrade(&["-k", "-b", BACKUP]).await?;
	let archive = tokio::fs::read(BACKUP).await?;
	let _ = tokio::fs::remove_file(BACKUP).await;
	let board = state.ubus.call("system", "board", serde_json::json!({})).await?;
	let hostname = board.get("hostname").and_then(|h| h.as_str()).unwrap_or("router");
	let disposition = format!("attachment; filename=\"{hostname}-backup.tar.gz\"");
	Ok((
		[
			(header::CONTENT_TYPE, "application/gzip".to_owned()),
			(header::CONTENT_DISPOSITION, disposition),
		],
		archive,
	))
}

fn reboot_soon(state: AppState) {
	tokio::spawn(async move {
		tokio::time::sleep(Duration::from_secs(2)).await;
		let _ = state.ubus.call("system", "reboot", serde_json::json!({})).await;
	});
}

pub async fn restore(State(state): State<AppState>, body: Body) -> Result<(), ApiError> {
	receive(body, RESTORE).await?;
	let restored = sysupgrade(&["-r", RESTORE]).await;
	let _ = tokio::fs::remove_file(RESTORE).await;
	restored?;
	reboot_soon(state);
	Ok(())
}

pub async fn firmware(body: Body) -> Result<(), ApiError> {
	receive(body, FIRMWARE).await?;
	if let Err(err) = sysupgrade(&["-T", FIRMWARE]).await {
		let _ = tokio::fs::remove_file(FIRMWARE).await;
		return Err(err);
	}
	tokio::spawn(async {
		tokio::time::sleep(Duration::from_secs(2)).await;
		let _ = Command::new("sysupgrade").arg(FIRMWARE).status().await;
	});
	Ok(())
}
