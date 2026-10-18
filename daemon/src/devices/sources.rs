use serde_json::{Value, json};
use tokio::fs;

use crate::platform::ubus::Ubus;

pub struct Lease {
	pub expires: u64,
	pub mac: String,
	pub ip: String,
	pub hostname: Option<String>,
}

pub async fn leases() -> Vec<Lease> {
	let raw = fs::read_to_string("/tmp/dhcp.leases").await.unwrap_or_default();
	raw.lines()
		.filter_map(|line| {
			let mut fields = line.split_whitespace();
			Some(Lease {
				expires: fields.next()?.parse().ok()?,
				mac: fields.next()?.to_ascii_lowercase(),
				ip: fields.next()?.to_owned(),
				hostname: fields.next().filter(|name| *name != "*").map(str::to_owned),
			})
		})
		.collect()
}

pub async fn local_macs() -> Vec<String> {
	let mut macs = Vec::new();
	let Ok(mut entries) = fs::read_dir("/sys/class/net").await else {
		return macs;
	};
	while let Ok(Some(entry)) = entries.next_entry().await {
		if let Ok(mac) = fs::read_to_string(entry.path().join("address")).await {
			macs.push(mac.trim().to_ascii_lowercase());
		}
	}
	macs
}

pub struct WifiClient {
	pub mac: String,
	pub iface: String,
	pub ssid: String,
	pub band: &'static str,
	pub signal: Option<i64>,
	pub rx_rate: u64,
	pub tx_rate: u64,
}

pub fn band(freq: u64) -> &'static str {
	match freq {
		0..=2500 => "2g",
		5925.. => "6g",
		_ => "5g",
	}
}

pub async fn access_points(ubus: &Ubus) -> Vec<String> {
	ubus.objects("hostapd.").await.unwrap_or_default()
}

pub async fn wifi_clients(ubus: &Ubus) -> Vec<WifiClient> {
	let mut clients = Vec::new();
	for object in access_points(ubus).await {
		let iface = object.trim_start_matches("hostapd.").to_owned();
		let status = ubus.call(&object, "get_status", json!({})).await.unwrap_or_default();
		let reply = ubus.call(&object, "get_clients", json!({})).await.unwrap_or_default();
		let ssid = status.get("ssid").and_then(Value::as_str).unwrap_or_default().to_owned();
		let band = band(reply.get("freq").and_then(Value::as_u64).unwrap_or_default());
		let Some(Value::Object(stations)) = reply.get("clients") else {
			continue;
		};
		for (mac, station) in stations {
			let rate = |direction: &str| {
				station
					.pointer(&format!("/rate/{direction}"))
					.and_then(Value::as_u64)
					.unwrap_or_default()
			};
			clients.push(WifiClient {
				mac: mac.to_ascii_lowercase(),
				iface: iface.clone(),
				ssid: ssid.clone(),
				band,
				signal: station.get("signal").and_then(Value::as_i64),
				rx_rate: rate("rx"),
				tx_rate: rate("tx"),
			});
		}
	}
	clients
}
