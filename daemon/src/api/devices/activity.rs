use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use crate::activity::{Counts, Site};
use crate::devices::normalize_mac;
use crate::http::{ApiError, ApiResult};
use crate::monitor::ledger::DayUsage;
use crate::platform::config::uci;
use crate::state::AppState;

const TOP_SITES: usize = 12;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityView {
	days: Vec<DayUsage>,
	sites: Vec<Site>,
	today: Counts,
}

async fn view(state: &AppState, macs: &[String]) -> ActivityView {
	let (sites, today) = state.activity.sites(macs, TOP_SITES);
	ActivityView {
		days: state.monitor.usage_days(macs).await,
		sites,
		today,
	}
}

pub async fn device(State(state): State<AppState>, Path(mac): Path<String>) -> ApiResult<ActivityView> {
	let mac = normalize_mac(&mac).ok_or_else(|| ApiError::bad_request("That doesn't look like a MAC address"))?;
	Ok(Json(view(&state, &[mac]).await))
}

pub async fn person(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<ActivityView> {
	let macs: Vec<String> = uci::sections_of(&state.ubus, "cowrt", "device")
		.await?
		.iter()
		.filter(|device| device.get("owner") == Some(id.as_str()))
		.filter_map(|device| device.get("mac").map(str::to_ascii_lowercase))
		.collect();
	Ok(Json(view(&state, &macs).await))
}
