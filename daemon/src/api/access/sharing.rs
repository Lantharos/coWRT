use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::access::ddns;
use crate::access::forwards::reach::{self, Reach};
use crate::access::forwards::upnp::{self, Upnp};
use crate::access::forwards::{self, Forward, ForwardInput};
use crate::api::devices::reservation;
use crate::devices::{self, normalize_mac};
use crate::http::{ApiError, ApiResult};
use crate::networks::Layout;
use crate::platform::config::Applied;
use crate::state::AppState;

const CONFIGS: &[&str] = &["cowrt", "firewall"];

#[derive(Serialize)]
pub struct Overview {
	forwards: Vec<Forward>,
	reach: Reach,
	upnp: Upnp,
	ddns: ddns::View,
}

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	Ok(Json(Overview {
		forwards: forwards::list(&state.ubus).await?,
		reach: reach::status(&state.ubus).await,
		upnp: upnp::status(&state.ubus).await?,
		ddns: ddns::view(&state.ubus).await?,
	}))
}

async fn ensure_reserved(state: &AppState, mac: &str) -> Result<(), ApiError> {
	let mac = normalize_mac(mac).ok_or_else(|| ApiError::bad_request("Pick a device"))?;
	let found = devices::inventory(&state.ubus, &Default::default(), None).await?;
	let device = found
		.into_iter()
		.find(|d| d.mac == mac)
		.ok_or_else(|| ApiError::not_found("That device isn't known"))?;
	if device.reserved_ip.is_some() {
		return Ok(());
	}
	let ip = device
		.ipv4
		.and_then(|ip| ip.parse().ok())
		.ok_or_else(|| ApiError::conflict("That device has no address yet"))?;
	reservation::set(state, &mac, Some(ip)).await
}

pub async fn create(State(state): State<AppState>, Json(input): Json<ForwardInput>) -> ApiResult<Applied> {
	input.validate()?;
	ensure_reserved(&state, &input.mac).await?;
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| {
			forwards::store(ubus, None, &input).await?;
			forwards::sync(ubus).await?;
			Ok(())
		})
		.await?;
	Ok(Json(applied))
}

pub async fn update(
	State(state): State<AppState>,
	Path(id): Path<String>,
	Json(input): Json<ForwardInput>,
) -> ApiResult<Applied> {
	input.validate()?;
	ensure_reserved(&state, &input.mac).await?;
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| {
			forwards::store(ubus, Some(&id), &input).await?;
			forwards::sync(ubus).await?;
			Ok(())
		})
		.await?;
	Ok(Json(applied))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, CONFIGS, None, async |ubus| {
			crate::platform::config::uci::delete(ubus, "cowrt", &id).await?;
			forwards::sync(ubus).await?;
			Ok(())
		})
		.await?;
	Ok(Json(applied))
}

#[derive(Deserialize)]
pub struct CheckInput {
	id: Option<String>,
}

#[derive(Serialize)]
pub struct CheckResult {
	open: bool,
}

pub async fn check(State(state): State<AppState>, Json(input): Json<CheckInput>) -> ApiResult<CheckResult> {
	let unreachable = |err: std::io::Error| ApiError::conflict(format!("The outside check didn't work: {err}"));
	let Some(id) = input.id else {
		return Ok(Json(CheckResult {
			open: reach::inbound(&state.ubus).await.map_err(unreachable)?,
		}));
	};
	let forward = forwards::list(&state.ubus)
		.await?
		.into_iter()
		.find(|f| f.id == id)
		.ok_or_else(|| ApiError::not_found("That forward no longer exists"))?;
	let port = forwards::first_port(&forward.external).ok_or_else(|| ApiError::bad_request("That forward has no port"))?;
	let probes = reach::check(&[port]).await.map_err(unreachable)?;
	Ok(Json(CheckResult {
		open: probes.first().is_some_and(|probe| probe.open),
	}))
}

#[derive(Deserialize)]
pub struct UpnpInput {
	enabled: bool,
}

pub async fn set_upnp(State(state): State<AppState>, Json(input): Json<UpnpInput>) -> Result<(), ApiError> {
	let layout = Layout::load(&state.ubus).await?;
	let trusted = layout
		.managed()
		.filter(|n| n.flag("trusted"))
		.map(|n| n.name.clone())
		.collect();
	upnp::set(&state.ubus, input.enabled, trusted).await
}

pub async fn set_ddns(State(state): State<AppState>, Json(input): Json<ddns::Input>) -> Result<(), ApiError> {
	if input.enabled && !input.hostname.contains('.') {
		return Err(ApiError::bad_request("Use a full hostname like home.example.com"));
	}
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(ddns::store(ubus, &input).await?)
		})
		.await?;
	ddns::refresh();
	Ok(())
}
