mod edit;
mod expiry;
pub mod roaming;
mod scan;
mod security;

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Value, json};

pub use edit::{NetworkInput, RadioInput, create_network, delete_network, update_network, update_radio};
pub use expiry::turn_off_expired;
pub use roaming::Roaming;
pub use scan::{Scan, scan};
pub use security::Security;

use crate::devices::sources;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Radio {
	pub name: String,
	pub band: String,
	pub channel: String,
	pub htmode: Option<String>,
	pub txpower: Option<u32>,
	pub country: Option<String>,
	pub enabled: bool,
	pub up: bool,
	pub live: Option<RadioLive>,
	pub channels: Vec<u32>,
	pub htmodes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadioLive {
	pub channel: u64,
	pub frequency: u64,
	pub txpower: Option<u64>,
	pub noise: Option<i64>,
	pub utilization: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
	pub id: String,
	pub ssid: String,
	pub security: Security,
	pub password: Option<String>,
	pub hidden: bool,
	pub isolate: bool,
	pub network: String,
	pub enabled: bool,
	pub bands: Vec<String>,
	pub clients: usize,
	pub expires: Option<u64>,
}

#[derive(Serialize)]
pub struct Overview {
	pub radios: Vec<Radio>,
	pub networks: Vec<Network>,
	pub roaming: Roaming,
}

fn band_code(band: &str) -> u64 {
	match band {
		"2g" => 2,
		"6g" => 6,
		"60g" => 60,
		_ => 5,
	}
}

async fn radio_live(ubus: &Ubus, iface: &str, band: &str) -> (Option<RadioLive>, Vec<u32>, Vec<String>) {
	let device = json!({ "device": iface });
	let hostapd = format!("hostapd.{iface}");
	let (info, freqs, status) = tokio::join!(
		ubus.call("iwinfo", "info", device.clone()),
		ubus.call("iwinfo", "freqlist", device.clone()),
		ubus.call(&hostapd, "get_status", json!({})),
	);
	let info = info.unwrap_or_default();
	let status = status.unwrap_or_default();
	let live = info.get("channel").and_then(Value::as_u64).map(|channel| RadioLive {
		channel,
		frequency: info.get("frequency").and_then(Value::as_u64).unwrap_or_default(),
		txpower: info.get("txpower").and_then(Value::as_u64),
		noise: info.get("noise").and_then(Value::as_i64),
		utilization: status.pointer("/airtime/utilization").and_then(Value::as_u64),
	});
	let code = band_code(band);
	let channels = freqs
		.unwrap_or_default()
		.get("results")
		.and_then(Value::as_array)
		.map(|list| {
			list.iter()
				.filter(|f| {
					f.get("band").and_then(Value::as_u64) == Some(code) && f.get("restricted") != Some(&Value::Bool(true))
				})
				.filter_map(|f| f.get("channel")?.as_u64().map(|c| c as u32))
				.collect()
		})
		.unwrap_or_default();
	let htmodes = info
		.get("htmodes")
		.and_then(Value::as_array)
		.map(|modes| modes.iter().filter_map(|m| m.as_str().map(str::to_owned)).collect())
		.unwrap_or_default();
	(live, channels, htmodes)
}

fn ifnames(status: &Value) -> HashMap<String, String> {
	let mut ifnames = HashMap::new();
	if let Value::Object(radios) = status {
		for radio in radios.values() {
			for iface in radio.get("interfaces").and_then(Value::as_array).into_iter().flatten() {
				if let (Some(section), Some(ifname)) = (
					iface.get("section").and_then(Value::as_str),
					iface.get("ifname").and_then(Value::as_str),
				) {
					ifnames.insert(section.to_owned(), ifname.to_owned());
				}
			}
		}
	}
	ifnames
}

fn radio_iface<'a>(sections: &[Section], ifnames: &'a HashMap<String, String>, radio: &str) -> Option<&'a String> {
	sections
		.iter()
		.filter(|s| s.kind == "wifi-iface" && s.get("device") == Some(radio))
		.find_map(|s| ifnames.get(&s.name))
}

pub async fn overview(ubus: &Ubus) -> Result<Overview, ubus::Error> {
	let sections = uci::sections(ubus, "wireless").await?;
	let status = ubus.call("network.wireless", "status", json!({})).await.unwrap_or_default();
	let ifnames = ifnames(&status);

	let devices: Vec<&Section> = sections.iter().filter(|s| s.kind == "wifi-device").collect();
	let mut radios = Vec::with_capacity(devices.len());
	for device in &devices {
		let band = device.get("band").unwrap_or("5g").to_owned();
		let iface = radio_iface(&sections, &ifnames, &device.name);
		let (live, channels, htmodes) = match iface {
			Some(iface) => radio_live(ubus, iface, &band).await,
			None => (None, Vec::new(), Vec::new()),
		};
		radios.push(Radio {
			name: device.name.clone(),
			channel: device.get("channel").unwrap_or("auto").to_owned(),
			htmode: device.get("htmode").map(str::to_owned),
			txpower: device.get("txpower").and_then(|p| p.parse().ok()),
			country: device.get("country").map(str::to_owned),
			enabled: !device.flag("disabled"),
			up: status
				.pointer(&format!("/{}/up", device.name))
				.and_then(Value::as_bool)
				.unwrap_or_default(),
			band,
			live,
			channels,
			htmodes,
		});
	}

	let clients = sources::wifi_clients(ubus).await;
	let mut networks: Vec<Network> = Vec::new();
	for iface in sections
		.iter()
		.filter(|s| s.kind == "wifi-iface" && s.get("mode").unwrap_or("ap") == "ap")
	{
		let ssid = iface.get("ssid").unwrap_or_default();
		let band = devices
			.iter()
			.find(|d| Some(d.name.as_str()) == iface.get("device"))
			.and_then(|d| d.get("band"));
		let connected = ifnames
			.get(&iface.name)
			.map_or(0, |ifname| clients.iter().filter(|c| &c.iface == ifname).count());
		if let Some(network) = networks.iter_mut().find(|n| n.ssid == ssid) {
			network.bands.extend(band.map(str::to_owned));
			network.clients += connected;
			network.enabled |= !iface.flag("disabled");
			continue;
		}
		networks.push(Network {
			id: iface.name.clone(),
			ssid: ssid.to_owned(),
			security: Security::from_uci(iface.get("encryption").unwrap_or("none")),
			password: iface.get("key").map(str::to_owned),
			hidden: iface.flag("hidden"),
			isolate: iface.flag("isolate"),
			network: iface.list("network").into_iter().next().unwrap_or_else(|| "lan".into()),
			enabled: !iface.flag("disabled"),
			bands: band.map(str::to_owned).into_iter().collect(),
			clients: connected,
			expires: iface.get("cowrt_expires").and_then(|at| at.parse().ok()),
		});
	}
	for network in &mut networks {
		network.bands.sort_by_key(|band| band_code(band));
		network.bands.dedup();
	}
	Ok(Overview {
		radios,
		networks,
		roaming: roaming::load(ubus).await?,
	})
}
