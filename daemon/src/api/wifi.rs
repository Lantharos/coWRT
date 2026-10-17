use axum::Json;
use axum::extract::{Path, State};

use crate::config::Applied;
use crate::http::ApiResult;
use crate::state::AppState;
use crate::wifi::{self, NetworkInput, Overview, RadioInput};

const CONFIG: &[&str] = &["wireless"];

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	Ok(Json(wifi::overview(&state.ubus).await?))
}

pub async fn create(State(state): State<AppState>, Json(input): Json<NetworkInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIG, None, async |ubus| {
			wifi::create_network(ubus, &input).await
		})
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
		.apply(&state.ubus, CONFIG, None, async |ubus| {
			wifi::update_network(ubus, &id, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIG, None, async |ubus| wifi::delete_network(ubus, &id).await)
		.await?;
	Ok(Json(applied))
}

pub async fn update_radio(
	State(state): State<AppState>,
	Path(name): Path<String>,
	Json(input): Json<RadioInput>,
) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIG, None, async |ubus| {
			wifi::update_radio(ubus, &name, &input).await
		})
		.await?;
	Ok(Json(applied))
}
