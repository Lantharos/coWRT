use axum::Json;
use axum::extract::State;
use serde::Serialize;
use serde_json::{Value, json};

use crate::config::uci;
use crate::http::ApiResult;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lan {
	device: Option<String>,
	ipv4: Option<String>,
	ipv6: Vec<String>,
	dhcp: Option<Dhcp>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dhcp {
	enabled: bool,
	start: u32,
	limit: u32,
	lease_time: String,
}

fn addresses(status: &Value, key: &str) -> Vec<String> {
	status
		.get(key)
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|entry| {
			Some(format!(
				"{}/{}",
				entry.get("address")?.as_str()?,
				entry.get("mask")?.as_u64()?
			))
		})
		.collect()
}

pub async fn lan(State(state): State<AppState>) -> ApiResult<Lan> {
	let status = state.ubus.call("network.interface.lan", "status", json!({})).await?;
	let dhcp = uci::sections_of(&state.ubus, "dhcp", "dhcp")
		.await?
		.into_iter()
		.find(|section| section.get("interface") == Some("lan"))
		.map(|section| Dhcp {
			enabled: !section.flag("ignore"),
			start: section.get("start").and_then(|v| v.parse().ok()).unwrap_or(100),
			limit: section.get("limit").and_then(|v| v.parse().ok()).unwrap_or(150),
			lease_time: section.get("leasetime").unwrap_or("12h").to_owned(),
		});
	let mut ipv6 = addresses(&status, "ipv6-address");
	ipv6.extend(addresses(&status, "ipv6-prefix-assignment"));
	Ok(Json(Lan {
		device: status.get("l3_device").and_then(Value::as_str).map(str::to_owned),
		ipv4: addresses(&status, "ipv4-address").into_iter().next(),
		ipv6,
		dhcp,
	}))
}
