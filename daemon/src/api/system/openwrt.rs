use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::http::{ApiError, ApiResult};
use crate::state::AppState;
use crate::system::upgrade::{self, Progress};

#[derive(Deserialize)]
struct Release {
	version: String,
}

#[derive(Deserialize)]
struct Board {
	release: Release,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWrtInfo {
	current: String,
	latest: Option<String>,
	available: bool,
	progress: Progress,
}

async fn current(state: &AppState) -> Result<String, ApiError> {
	let board: Board = serde_json::from_value(state.ubus.call("system", "board", serde_json::json!({})).await?)
		.map_err(|err| ApiError::conflict(format!("Couldn't read the router's version: {err}")))?;
	Ok(board.release.version)
}

pub async fn check_openwrt(State(state): State<AppState>) -> ApiResult<OpenWrtInfo> {
	let current = current(&state).await?;
	let latest = state.upgrade.latest().await.ok();
	Ok(Json(OpenWrtInfo {
		available: latest.as_deref().is_some_and(|latest| upgrade::newer(latest, &current)),
		current,
		latest,
		progress: state.upgrade.progress(),
	}))
}

pub async fn upgrade_openwrt(State(state): State<AppState>) -> Result<(), ApiError> {
	let current = current(&state).await?;
	let latest = state
		.upgrade
		.latest()
		.await
		.map_err(|err| ApiError::conflict(format!("Couldn't reach OpenWrt: {err}")))?;
	if !upgrade::newer(&latest, &current) {
		return Err(ApiError::conflict("You already have the newest OpenWrt"));
	}
	if !state.upgrade.begin() {
		return Err(ApiError::conflict("An update is already running"));
	}
	tokio::spawn(async move { state.upgrade.run(&latest).await });
	Ok(())
}
