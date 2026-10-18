pub mod sources;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;

use serde::Serialize;

use crate::config::uci;
use crate::monitor::DeviceUsage;
use crate::networks::Layout;
use crate::policy::Controls;
use crate::ubus::{self, Ubus};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
	pub mac: String,
	pub name: String,
	pub custom_name: Option<String>,
	pub hostname: Option<String>,
	pub icon: Option<String>,
	pub ipv4: Option<String>,
	pub ipv6: Vec<String>,
	pub online: bool,
	pub connection: Connection,
	pub reserved_ip: Option<String>,
	pub lease_expires: Option<u64>,
	pub is_you: bool,
	pub network: Option<String>,
	pub controls: Controls,
	pub usage: DeviceUsage,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Connection {
	#[serde(rename_all = "camelCase")]
	Wifi {
		ssid: String,
		band: &'static str,
		iface: String,
		signal: Option<i64>,
		rx_rate: u64,
		tx_rate: u64,
	},
	Wired,
	Offline,
}

pub fn normalize_mac(mac: &str) -> Option<String> {
	let mac = mac.to_ascii_lowercase().replace('-', ":");
	let valid = mac.len() == 17
		&& mac
			.split(':')
			.all(|octet| octet.len() == 2 && octet.chars().all(|c| c.is_ascii_hexdigit()));
	valid.then_some(mac)
}

pub fn section_id(mac: &str) -> String {
	mac.replace(':', "")
}

impl Device {
	fn new(mac: &str) -> Self {
		Self {
			mac: mac.to_owned(),
			name: String::new(),
			custom_name: None,
			hostname: None,
			icon: None,
			ipv4: None,
			ipv6: Vec::new(),
			online: false,
			connection: Connection::Offline,
			reserved_ip: None,
			lease_expires: None,
			is_you: false,
			network: None,
			controls: Controls::default(),
			usage: DeviceUsage::default(),
		}
	}
}

struct Found {
	devices: BTreeMap<String, Device>,
	local: HashSet<String>,
}

impl Found {
	fn entry(&mut self, mac: &str) -> Option<&mut Device> {
		let mac = normalize_mac(mac)?;
		if self.local.contains(&mac) {
			return None;
		}
		Some(self.devices.entry(mac.clone()).or_insert_with(|| Device::new(&mac)))
	}
}

pub async fn inventory(
	ubus: &Ubus,
	usage: &HashMap<String, DeviceUsage>,
	requester: Option<IpAddr>,
) -> Result<Vec<Device>, ubus::Error> {
	let (leases, neighbors, wifi, local, customs, hosts, wan) = tokio::join!(
		sources::leases(),
		tokio::task::spawn_blocking(crate::netlink::neighbors::list),
		sources::wifi_clients(ubus),
		sources::local_macs(),
		uci::sections_of(ubus, "cowrt", "device"),
		uci::sections_of(ubus, "dhcp", "host"),
		ubus.call("network.interface.wan", "status", serde_json::json!({})),
	);
	let wan_device = wan
		.ok()
		.and_then(|status| Some(status.get("l3_device")?.as_str()?.to_owned()));
	let mut found = Found {
		devices: BTreeMap::new(),
		local: local.into_iter().collect(),
	};

	for lease in leases {
		if let Some(device) = found.entry(&lease.mac) {
			device.ipv4 = Some(lease.ip);
			device.hostname = lease.hostname;
			device.lease_expires = Some(lease.expires);
		}
	}
	for host in hosts? {
		for mac in host.list("mac") {
			if let Some(device) = found.entry(&mac) {
				device.reserved_ip = host.get("ip").map(str::to_owned);
				device.hostname = device.hostname.take().or_else(|| host.get("name").map(str::to_owned));
			}
		}
	}
	for custom in customs? {
		if let Some(device) = custom.get("mac").and_then(|mac| found.entry(mac)) {
			device.custom_name = custom.get("name").map(str::to_owned);
			device.icon = custom.get("icon").map(str::to_owned);
			device.controls = Controls::from_section(&custom);
		}
	}
	let neighbors = neighbors.ok().and_then(Result::ok).unwrap_or_default();
	for neighbor in neighbors.into_iter().filter(|n| Some(&n.device) != wan_device.as_ref()) {
		let Some(device) = found.entry(&neighbor.mac) else { continue };
		match neighbor.ip {
			IpAddr::V4(ip) => {
				device.ipv4.get_or_insert(ip.to_string());
			}
			IpAddr::V6(ip) if !ip.is_unicast_link_local() => device.ipv6.push(ip.to_string()),
			IpAddr::V6(_) => {}
		}
		if neighbor.active {
			device.online = true;
			device.connection = Connection::Wired;
		}
	}
	for client in wifi {
		if let Some(device) = found.entry(&client.mac) {
			device.online = true;
			device.connection = Connection::Wifi {
				ssid: client.ssid,
				band: client.band,
				iface: client.iface,
				signal: client.signal,
				rx_rate: client.rx_rate,
				tx_rate: client.tx_rate,
			};
		}
	}

	let subnets = Layout::load(ubus).await?.subnets();
	let requester = requester.map(|ip| ip.to_string());
	let mut devices: Vec<Device> = found
		.devices
		.into_values()
		.map(|mut device| {
			device.is_you = requester.is_some() && device.ipv4 == requester;
			device.usage = usage.get(&device.mac).cloned().unwrap_or_default();
			device.network = device
				.ipv4
				.as_deref()
				.and_then(|ip| ip.parse().ok())
				.and_then(|ip| subnets.iter().find(|(_, cidr)| cidr.contains(ip)))
				.map(|(id, _)| id.clone());
			device.name = device
				.custom_name
				.clone()
				.or_else(|| device.hostname.clone())
				.unwrap_or_else(|| device.mac.to_uppercase());
			device
		})
		.collect();
	devices.sort_by(|a, b| {
		b.online
			.cmp(&a.online)
			.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
	});
	Ok(devices)
}
