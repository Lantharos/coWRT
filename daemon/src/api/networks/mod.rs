pub mod lan;

use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};

use crate::http::ApiResult;
use crate::networks::{self, NetworkInput, Overview, PortInput};
use crate::platform::config::Applied;
use crate::state::AppState;

const CONFIGS: &[&str] = &["network", "dhcp", "firewall", "cowrt"];
const ROLLBACK: Option<Duration> = Some(Duration::from_secs(60));

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	Ok(Json(networks::overview(&state.ubus).await?))
}

pub async fn create(State(state): State<AppState>, Json(input): Json<NetworkInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| networks::create(ubus, &input).await)
		.await?;
	Ok(Json(applied))
}

pub async fn update(
	State(state): State<AppState>,
	Path(id): Path<String>,
	Json(input): Json<NetworkInput>,
) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, ROLLBACK, async |ubus| {
			networks::update(ubus, &id, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, ROLLBACK, async |ubus| networks::delete(ubus, &id).await)
		.await?;
	Ok(Json(applied))
}

pub async fn update_port(
	State(state): State<AppState>,
	Path(name): Path<String>,
	Json(input): Json<PortInput>,
) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, ROLLBACK, async |ubus| {
			networks::update_port(ubus, &name, &input).await
		})
		.await?;
	Ok(Json(applied))
}
