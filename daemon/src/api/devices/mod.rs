mod controls;
pub mod people;
pub mod reservation;

use std::net::{Ipv4Addr, SocketAddr};

use axum::Json;
use axum::extract::{ConnectInfo, Path, State};
use serde::{Deserialize, Deserializer};
use serde_json::json;

use crate::devices::policy::{Priority, Schedule};
use crate::devices::{self, Connection, Device, normalize_mac, section_id};
use crate::http::{ApiError, ApiResult};
use crate::platform::config::uci;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> ApiResult<Vec<Device>> {
	let usage = state.monitor.device_usage().await;
	let mut signals = state.monitor.signals();
	let mut devices = devices::inventory(&state.ubus, &usage, Some(peer.ip().to_canonical())).await?;
	for device in &mut devices {
		device.signals = signals.remove(&device.mac).unwrap_or_default();
	}
	Ok(Json(devices))
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<Option<T>, D::Error> {
	T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceUpdate {
	#[serde(default, deserialize_with = "present")]
	name: Option<Option<String>>,
	#[serde(default, deserialize_with = "present")]
	icon: Option<Option<String>>,
	#[serde(default, deserialize_with = "present")]
	reserved_ip: Option<Option<Ipv4Addr>>,
	blocked: Option<bool>,
	schedules: Option<Vec<Schedule>>,
	#[serde(default, deserialize_with = "present")]
	limit_down: Option<Option<u32>>,
	#[serde(default, deserialize_with = "present")]
	limit_up: Option<Option<u32>>,
	priority: Option<Priority>,
	#[serde(default, deserialize_with = "present")]
	owner: Option<Option<String>>,
}

fn mac_param(mac: &str) -> Result<String, ApiError> {
	normalize_mac(mac).ok_or_else(|| ApiError::bad_request("That doesn't look like a MAC address"))
}

async fn set_label(state: &AppState, mac: &str, body: &DeviceUpdate) -> Result<(), ApiError> {
	let section = format!("d{}", section_id(mac));
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			uci::upsert(ubus, "cowrt", "device", &section, json!({ "mac": mac })).await?;
			for (option, value) in [("name", &body.name), ("icon", &body.icon)] {
				match value {
					Some(Some(value)) if !value.trim().is_empty() => {
						uci::set(ubus, "cowrt", &section, json!({ option: value.trim() })).await?
					}
					Some(_) => uci::unset(ubus, "cowrt", &section, &[option]).await?,
					None => {}
				}
			}
			Ok(())
		})
		.await?;
	Ok(())
}

pub async fn update(
	State(state): State<AppState>,
	Path(mac): Path<String>,
	Json(body): Json<DeviceUpdate>,
) -> Result<(), ApiError> {
	let mac = mac_param(&mac)?;
	if body.name.is_some() || body.icon.is_some() {
		set_label(&state, &mac, &body).await?;
	}
	if let Some(reserved) = body.reserved_ip {
		reservation::set(&state, &mac, reserved).await?;
	}
	if let Some(owner) = &body.owner {
		people::assign(&state, &mac, owner.as_deref()).await?;
	}
	controls::set(&state, &mac, &body).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disconnect {
	#[serde(default)]
	block_minutes: u32,
}

pub async fn disconnect(
	State(state): State<AppState>,
	Path(mac): Path<String>,
	Json(body): Json<Disconnect>,
) -> Result<(), ApiError> {
	let mac = mac_param(&mac)?;
	let devices = devices::inventory(&state.ubus, &Default::default(), None).await?;
	let Some(Connection::Wifi { iface, .. }) = devices.into_iter().find(|d| d.mac == mac).map(|d| d.connection) else {
		return Err(ApiError::conflict("This device isn't connected over Wi-Fi"));
	};
	let ban_ms = u64::from(body.block_minutes) * 60_000;
	let args = json!({ "addr": mac, "reason": 5, "deauth": true, "ban_time": ban_ms });
	state.ubus.call(&format!("hostapd.{iface}"), "del_client", args).await?;
	Ok(())
}
