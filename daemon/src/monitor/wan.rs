use serde::Serialize;
use serde_json::{Value, json};

use crate::ubus::Ubus;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wan {
	pub up: bool,
	pub device: Option<String>,
	pub proto: Option<String>,
	pub uptime: u64,
	pub ipv4: Option<String>,
	pub gateway: Option<String>,
	pub ipv6: Option<String>,
	pub prefix6: Option<String>,
	pub dns: Vec<String>,
}

fn address(entry: &Value) -> Option<String> {
	Some(format!(
		"{}/{}",
		entry.get("address")?.as_str()?,
		entry.get("mask")?.as_u64()?
	))
}

fn first(status: &Value, key: &str) -> Option<String> {
	status.get(key)?.as_array()?.iter().find_map(address)
}

impl Wan {
	pub async fn read(ubus: &Ubus) -> Self {
		let v4 = ubus
			.call("network.interface.wan", "status", json!({}))
			.await
			.unwrap_or_default();
		let v6 = ubus
			.call("network.interface.wan6", "status", json!({}))
			.await
			.unwrap_or_default();
		let gateway = v4.get("route").and_then(Value::as_array).and_then(|routes| {
			routes
				.iter()
				.find(|route| route.get("target").and_then(Value::as_str) == Some("0.0.0.0"))
				.and_then(|route| Some(route.get("nexthop")?.as_str()?.to_owned()))
		});
		let dns = [&v4, &v6]
			.iter()
			.filter_map(|status| status.get("dns-server")?.as_array())
			.flatten()
			.filter_map(|server| server.as_str().map(str::to_owned))
			.collect();
		Self {
			up: v4.get("up").and_then(Value::as_bool).unwrap_or_default(),
			device: v4.get("l3_device").and_then(Value::as_str).map(str::to_owned),
			proto: v4.get("proto").and_then(Value::as_str).map(str::to_owned),
			uptime: v4.get("uptime").and_then(Value::as_u64).unwrap_or_default(),
			ipv4: first(&v4, "ipv4-address"),
			gateway,
			ipv6: first(&v6, "ipv6-address"),
			prefix6: first(&v6, "ipv6-prefix"),
			dns,
		}
	}
}

pub async fn utc_offset(ubus: &Ubus) -> Option<i64> {
	let info = ubus.call("system", "info", json!({})).await.ok()?;
	let local = info.get("localtime")?.as_i64()?;
	Some(local - crate::state::unix_now() as i64)
}
