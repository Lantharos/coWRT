use std::net::Ipv4Addr;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::devices::{self, normalize_mac};
use crate::http::{ApiError, ApiResult};
use crate::networks::lan::{self, Ipv6, LocalName, LocalNameInput, Route, RouteInput};
use crate::platform::config::Applied;
use crate::state::AppState;

#[derive(Serialize)]
pub struct Overview {
	domain: Option<String>,
	names: Vec<LocalName>,
	routes: Vec<Route>,
	ipv6: Ipv6,
}

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	let ubus = &state.ubus;
	Ok(Json(Overview {
		domain: lan::domain(ubus).await?,
		names: lan::names(ubus).await?,
		routes: lan::routes(ubus).await?,
		ipv6: lan::ipv6_status(ubus).await?,
	}))
}

pub async fn create_name(State(state): State<AppState>, Json(input): Json<LocalNameInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["dhcp"], None, async |ubus| {
			lan::create_name(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn delete_name(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["dhcp"], None, async |ubus| lan::delete_name(ubus, &id).await)
		.await?;
	Ok(Json(applied))
}

pub async fn create_route(State(state): State<AppState>, Json(input): Json<RouteInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["network"], None, async |ubus| {
			lan::create_route(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn delete_route(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["network"], None, async |ubus| {
			lan::delete_route(ubus, &id).await
		})
		.await?;
	Ok(Json(applied))
}

#[derive(Deserialize)]
pub struct Ipv6Input {
	enabled: bool,
}

pub async fn set_ipv6(State(state): State<AppState>, Json(input): Json<Ipv6Input>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["network", "dhcp"], None, async |ubus| {
			lan::set_ipv6(ubus, input.enabled).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn wake(State(state): State<AppState>, Path(mac): Path<String>) -> Result<(), ApiError> {
	let mac = normalize_mac(&mac).ok_or_else(|| ApiError::bad_request("That doesn't look like a MAC address"))?;
	let devices = devices::inventory(&state.ubus, &Default::default(), None).await?;
	let ip: Ipv4Addr = devices
		.iter()
		.find(|d| d.mac == mac)
		.and_then(|d| d.reserved_ip.as_deref().or(d.ipv4.as_deref()))
		.and_then(|ip| ip.parse().ok())
		.ok_or_else(|| ApiError::conflict("The router doesn't know which network this device is on"))?;
	lan::wake(&state.ubus, &mac, ip).await
}
