use axum::Json;
use axum::extract::{Path, State};

use crate::http::{ApiError, ApiResult};
use crate::internet::uplinks::{self, Kind, Settings, Status, tether};
use crate::state::AppState;

pub async fn status(State(state): State<AppState>) -> Json<Status> {
	Json(state.uplinks.status())
}

pub async fn update_failover(State(state): State<AppState>, Json(input): Json<Settings>) -> Result<(), ApiError> {
	let known = state.uplinks.status().uplinks.iter().any(|u| u.id == input.mode);
	if input.mode != "auto" && !known {
		return Err(ApiError::bad_request("Pick one of your internet connections"));
	}
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(uplinks::store_settings(ubus, &input).await?)
		})
		.await?;
	state.uplinks.nudge();
	Ok(())
}

pub async fn add_phone(State(state): State<AppState>) -> ApiResult<Status> {
	if state.uplinks.status().uplinks.iter().any(|u| u.kind == Kind::Phone) {
		return Err(ApiError::conflict("A phone is already set up as a backup"));
	}
	tether::install_drivers().await?;
	let device = tether::plugged_in().await.unwrap_or_else(|| "usb0".to_owned());
	state
		.applier
		.apply(&state.ubus, &["network", "firewall", "cowrt"], None, async |ubus| {
			tether::create(ubus, &device).await?;
			Ok(uplinks::add(ubus, tether::PHONE, "Phone", Kind::Phone).await?)
		})
		.await?;
	uplinks::retire_mwan3().await;
	state.uplinks.nudge();
	Ok(Json(state.uplinks.status()))
}

pub async fn remove(State(state): State<AppState>, Path(id): Path<String>) -> Result<(), ApiError> {
	if id == crate::monitor::wan::MAIN {
		return Err(ApiError::conflict("Your main line can't be removed"));
	}
	if !state.uplinks.status().uplinks.iter().any(|u| u.id == id) {
		return Err(ApiError::not_found("That connection doesn't exist"));
	}
	state
		.applier
		.apply(&state.ubus, &["network", "firewall", "cowrt"], None, async |ubus| {
			tether::remove(ubus, &id).await?;
			Ok(uplinks::forget(ubus, &id).await?)
		})
		.await?;
	state.uplinks.nudge();
	Ok(())
}
