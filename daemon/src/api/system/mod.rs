mod backup;
mod logs;
mod openwrt;
mod settings;
pub mod setup;
mod update;

pub use backup::{backup, firmware, restore};
pub use logs::logs;
pub use openwrt::{check_openwrt, upgrade_openwrt};
pub use settings::{password, reboot, update_settings};
pub use update::{check_update, install_update, update_automatic};

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::monitor::History;
use crate::state::AppState;

#[derive(Deserialize)]
struct Release {
	version: String,
	revision: String,
	target: String,
}

#[derive(Deserialize)]
struct Board {
	hostname: String,
	model: String,
	system: String,
	kernel: String,
	release: Release,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
	hostname: String,
	model: String,
	processor: String,
	kernel: String,
	openwrt: String,
	revision: String,
	target: String,
	cowrt: &'static str,
	zonename: Option<String>,
	storage: Storage,
	local_time: i64,
}

#[derive(Serialize)]
pub struct Storage {
	used: u64,
	total: u64,
}

pub async fn info(State(state): State<AppState>) -> ApiResult<SystemInfo> {
	let board: Board = state.ubus.call_as("system", "board", json!({})).await?;
	let info = state.ubus.call("system", "info", json!({})).await?;
	let kib = |key: &str| {
		info.pointer(&format!("/root/{key}"))
			.and_then(Value::as_u64)
			.unwrap_or_default()
			* 1024
	};
	Ok(Json(SystemInfo {
		hostname: board.hostname,
		model: board.model,
		processor: board.system,
		kernel: board.kernel,
		openwrt: board.release.version,
		revision: board.release.revision,
		target: board.release.target,
		cowrt: env!("CARGO_PKG_VERSION"),
		zonename: crate::platform::config::uci::sections_of(&state.ubus, "system", "system")
			.await?
			.first()
			.and_then(|s| s.get("zonename").map(str::to_owned)),
		storage: Storage {
			used: kib("used"),
			total: kib("total"),
		},
		local_time: info.get("localtime").and_then(Value::as_i64).unwrap_or_default(),
	}))
}

pub async fn history(State(state): State<AppState>) -> Json<History> {
	Json(state.monitor.history())
}
