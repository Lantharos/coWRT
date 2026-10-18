mod address;
mod adopt;
mod edit;
pub mod firewall;
pub mod lan;
mod layout;
pub mod ports;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use address::Cidr;
pub use adopt::adopt;
pub use edit::{NetworkInput, PortInput, create, delete, update, update_port};
pub use layout::Layout;
use layout::PRIMARY;
use ports::Port;

use crate::platform::ubus::{self, Ubus};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dhcp {
	pub enabled: bool,
	pub start: u32,
	pub limit: u32,
	pub lease_time: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
	pub id: String,
	pub name: String,
	pub color: String,
	pub primary: bool,
	pub vlan: u16,
	pub address: Option<String>,
	pub dhcp: Option<Dhcp>,
	pub trusted: bool,
	pub internet: bool,
	pub ports: Vec<String>,
	pub tagged: Vec<String>,
	pub wifi: Vec<String>,
	pub ipv6: NetworkIpv6,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkIpv6 {
	pub enabled: bool,
	pub prefixes: Vec<String>,
}

#[derive(Serialize)]
pub struct Overview {
	pub networks: Vec<Network>,
	pub ports: Vec<Port>,
}

async fn assigned_prefixes(ubus: &Ubus) -> HashMap<String, Vec<String>> {
	let dump = ubus.call("network.interface", "dump", json!({})).await.unwrap_or_default();
	dump.get("interface")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|interface| {
			let id = interface.get("interface")?.as_str()?.to_owned();
			let prefixes = interface
				.get("ipv6-prefix-assignment")?
				.as_array()?
				.iter()
				.filter_map(|p| Some(format!("{}/{}", p.get("address")?.as_str()?, p.get("mask")?.as_u64()?)))
				.collect();
			Some((id, prefixes))
		})
		.collect()
}

pub async fn overview(ubus: &Ubus) -> Result<Overview, ubus::Error> {
	let layout = Layout::load(ubus).await?;
	let assigned = assigned_prefixes(ubus).await;
	let mut networks = Vec::new();
	for managed in layout.managed() {
		let id = &managed.name;
		let Some(vlan) = layout.vlan_of(id) else { continue };
		let members = layout.members(vlan);
		let mut wifi: Vec<String> = layout
			.wireless
			.iter()
			.filter(|s| s.kind == "wifi-iface" && s.list("network").iter().any(|n| n == id))
			.filter_map(|s| s.get("ssid").map(str::to_owned))
			.collect();
		wifi.dedup();
		networks.push(Network {
			id: id.clone(),
			name: managed.get("name").unwrap_or(id).to_owned(),
			color: managed.get("color").unwrap_or("green").to_owned(),
			primary: id == PRIMARY,
			vlan,
			address: layout.address(id).map(|cidr| cidr.to_string()),
			dhcp: layout.dhcp(id).map(|section| Dhcp {
				enabled: !section.flag("ignore"),
				start: section.get("start").and_then(|v| v.parse().ok()).unwrap_or(100),
				limit: section.get("limit").and_then(|v| v.parse().ok()).unwrap_or(150),
				lease_time: section.get("leasetime").unwrap_or("12h").to_owned(),
			}),
			trusted: managed.flag("trusted"),
			internet: managed.flag("internet"),
			ports: members.iter().filter(|m| m.untagged).map(|m| m.port.clone()).collect(),
			tagged: members.iter().filter(|m| !m.untagged).map(|m| m.port.clone()).collect(),
			wifi,
			ipv6: NetworkIpv6 {
				enabled: layout.interface(id).is_some_and(|i| i.get("ip6assign").is_some()),
				prefixes: assigned.get(id.as_str()).cloned().unwrap_or_default(),
			},
		});
	}
	networks.sort_by_key(|network| (!network.primary, network.vlan));

	let mut detected = Vec::new();
	for name in ports::detect(&layout.wan_devices(), &layout.bridge_ports()).await {
		let (up, speed) = ports::link(&name).await;
		detected.push(Port {
			label: ports::label(&name),
			up,
			speed,
			network: networks.iter().find(|n| n.ports.contains(&name)).map(|n| n.id.clone()),
			tagged: networks
				.iter()
				.filter(|n| n.tagged.contains(&name))
				.map(|n| n.id.clone())
				.collect(),
			name,
		});
	}
	Ok(Overview {
		networks,
		ports: detected,
	})
}
