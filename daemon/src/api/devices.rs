use std::net::{Ipv4Addr, SocketAddr};

use axum::Json;
use axum::extract::{ConnectInfo, Path, State};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};

use crate::config::uci;
use crate::devices::{self, Connection, Device, normalize_mac, section_id};
use crate::http::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn list(State(state): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> ApiResult<Vec<Device>> {
	Ok(Json(devices::inventory(&state.ubus, Some(peer.ip().to_canonical())).await?))
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
}

fn mac_param(mac: &str) -> Result<String, ApiError> {
	normalize_mac(mac).ok_or_else(|| ApiError::bad_request("That doesn't look like a MAC address"))
}

fn dns_name(name: &str) -> String {
	let label: String = name
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() {
				c.to_ascii_lowercase()
			} else {
				'-'
			}
		})
		.collect();
	label.trim_matches('-').chars().take(63).collect()
}

async fn lan_subnet(state: &AppState) -> Option<(Ipv4Addr, u32)> {
	let status = state.ubus.call("network.interface.lan", "status", json!({})).await.ok()?;
	let entry = status.get("ipv4-address")?.as_array()?.first()?;
	Some((
		entry.get("address")?.as_str()?.parse().ok()?,
		entry.get("mask")?.as_u64()? as u32,
	))
}

async fn validate_reservation(state: &AppState, mac: &str, ip: Ipv4Addr) -> Result<(), ApiError> {
	let (router, prefix) = lan_subnet(state)
		.await
		.ok_or_else(|| ApiError::conflict("The local network has no IPv4 address"))?;
	let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
	let (net, host) = (u32::from(router) & mask, u32::from(ip));
	if host & mask != net || ip == router || host == net || host == net | !mask {
		return Err(ApiError::bad_request(format!(
			"Pick an address inside {}/{prefix} that isn't the router",
			Ipv4Addr::from(net)
		)));
	}
	let taken = uci::sections_of(&state.ubus, "dhcp", "host")
		.await?
		.into_iter()
		.any(|host| host.get("ip") == Some(&ip.to_string()) && !host.list("mac").iter().any(|m| m.eq_ignore_ascii_case(mac)));
	if taken {
		return Err(ApiError::conflict("Another device already has that address reserved"));
	}
	Ok(())
}

pub async fn update(
	State(state): State<AppState>,
	Path(mac): Path<String>,
	Json(body): Json<DeviceUpdate>,
) -> Result<(), ApiError> {
	let mac = mac_param(&mac)?;
	let id = section_id(&mac);

	if body.name.is_some() || body.icon.is_some() {
		state
			.applier
			.apply(&state.ubus, &["cowrt"], None, async |ubus| {
				let section = format!("d{id}");
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
	}

	if let Some(reservation) = body.reserved_ip {
		if let Some(ip) = reservation {
			validate_reservation(&state, &mac, ip).await?;
		}
		let devices = devices::inventory(&state.ubus, None).await?;
		let name = devices
			.iter()
			.find(|d| d.mac == mac)
			.map(|d| dns_name(&d.name))
			.filter(|n| !n.is_empty());
		state
			.applier
			.apply(&state.ubus, &["dhcp"], None, async |ubus| {
				let section = format!("cowrt{id}");
				let existing = uci::sections_of(ubus, "dhcp", "host").await?;
				for host in existing
					.iter()
					.filter(|h| h.name != section && h.list("mac").iter().any(|m| m.eq_ignore_ascii_case(&mac)))
				{
					uci::delete(ubus, "dhcp", &host.name).await?;
				}
				match reservation {
					Some(ip) => {
						let mut values = json!({ "mac": mac, "ip": ip.to_string(), "dns": "1" });
						if let Some(name) = &name {
							values["name"] = Value::from(name.as_str());
						}
						uci::upsert(ubus, "dhcp", "host", &section, values).await?;
					}
					None if existing.iter().any(|h| h.name == section) => uci::delete(ubus, "dhcp", &section).await?,
					None => {}
				}
				Ok(())
			})
			.await?;
	}
	Ok(())
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
	let devices = devices::inventory(&state.ubus, None).await?;
	let Some(Connection::Wifi { iface, .. }) = devices.into_iter().find(|d| d.mac == mac).map(|d| d.connection) else {
		return Err(ApiError::conflict("This device isn't connected over Wi-Fi"));
	};
	let ban_ms = u64::from(body.block_minutes) * 60_000;
	state
		.ubus
		.call(
			&format!("hostapd.{iface}"),
			"del_client",
			json!({ "addr": mac, "reason": 5, "deauth": true, "ban_time": ban_ms }),
		)
		.await?;
	Ok(())
}
