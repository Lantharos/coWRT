use std::time::Duration;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::config::Applied;
use crate::http::{ApiError, ApiResult};
use crate::state::AppState;
use crate::vpn::{client, server};

const CONFIGS: &[&str] = &["network", "firewall", "cowrt", "dhcp"];

#[derive(Serialize)]
pub struct Overview {
	server: server::View,
	client: client::View,
}

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	Ok(Json(Overview {
		server: server::view(&state.ubus).await?,
		client: client::view(&state.ubus).await?,
	}))
}

pub async fn update_server(State(state): State<AppState>, Json(input): Json<server::Input>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| server::store(ubus, &input).await)
		.await?;
	Ok(Json(applied))
}

#[derive(Deserialize)]
pub struct PeerInput {
	name: String,
}

pub async fn add_peer(State(state): State<AppState>, Json(input): Json<PeerInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| {
			server::add_peer(ubus, &input.name).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn remove_peer(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| server::remove_peer(ubus, &id).await)
		.await?;
	Ok(Json(applied))
}

#[derive(Deserialize)]
pub struct ConfigQuery {
	#[serde(default)]
	everything: bool,
}

pub async fn peer_config(
	State(state): State<AppState>,
	Path(id): Path<String>,
	Query(query): Query<ConfigQuery>,
) -> Result<impl IntoResponse, ApiError> {
	let config = server::peer_config(&state.ubus, &id, query.everything).await?;
	Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], config))
}

pub async fn update_client(State(state): State<AppState>, Json(input): Json<client::Input>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, Some(Duration::from_secs(90)), async |ubus| {
			client::store(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}
