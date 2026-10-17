use std::net::Ipv4Addr;

use serde_json::json;

use super::address::Cidr;
use super::edit;
use super::firewall;
use super::layout::{DEFAULT_BRIDGE, Layout, Membership, PRIMARY};
use crate::config::{self, uci};
use crate::ubus::{self, Ubus};

const PRIMARY_VLAN: u16 = 1;
const STOCK_ADDRESS: Cidr = Cidr {
	addr: Ipv4Addr::new(192, 168, 1, 1),
	prefix: 24,
};
const DEFAULT_ADDRESSES: [Cidr; 2] = [
	Cidr {
		addr: Ipv4Addr::new(10, 0, 0, 1),
		prefix: 24,
	},
	Cidr {
		addr: Ipv4Addr::new(10, 10, 0, 1),
		prefix: 24,
	},
];
const CONFIGS: [&str; 4] = ["network", "firewall", "dhcp", "cowrt"];

async fn default_address(ubus: &Ubus) -> Cidr {
	let wan = ubus
		.call("network.interface.wan", "status", json!({}))
		.await
		.unwrap_or_default();
	let upstream: Option<Cidr> = wan.pointer("/ipv4-address/0").and_then(|entry| {
		let address = entry.get("address")?.as_str()?;
		format!("{address}/{}", entry.get("mask")?.as_u64()?).parse().ok()
	});
	DEFAULT_ADDRESSES
		.into_iter()
		.find(|candidate| upstream.is_none_or(|wan| !wan.overlaps(*candidate)))
		.unwrap_or(DEFAULT_ADDRESSES[1])
}

async fn ensure_bridge(ubus: &Ubus, layout: &Layout) -> Result<String, ubus::Error> {
	if let Some(bridge) = layout.bridge().filter(|b| b.get("type") == Some("bridge")) {
		uci::set(ubus, "network", &bridge.name, json!({ "vlan_filtering": "1" })).await?;
		return Ok(bridge.get("name").unwrap_or(DEFAULT_BRIDGE).to_owned());
	}
	let ports: Vec<String> = layout
		.interface(PRIMARY)
		.and_then(|lan| lan.get("device"))
		.map(str::to_owned)
		.into_iter()
		.collect();
	let bridge = json!({ "name": DEFAULT_BRIDGE, "type": "bridge", "ports": ports, "vlan_filtering": "1" });
	uci::add(ubus, "network", "device", None, bridge).await?;
	Ok(DEFAULT_BRIDGE.to_owned())
}

async fn retire_stock_firewall(ubus: &Ubus) -> Result<(), ubus::Error> {
	for section in uci::sections(ubus, "firewall").await? {
		let stock_zone = section.kind == "zone" && section.get("name") == Some(PRIMARY);
		let stock_forwarding = section.kind == "forwarding" && section.get("src") == Some(PRIMARY);
		if stock_zone || stock_forwarding {
			uci::delete(ubus, "firewall", &section.name).await?;
		}
	}
	Ok(())
}

async fn convert(ubus: &Ubus) -> Result<(), ubus::Error> {
	let layout = Layout::load(ubus).await?;
	let Some(lan) = layout.interface(PRIMARY) else {
		return Ok(());
	};
	let ports = layout.bridge_ports();
	let ports = if ports.is_empty() {
		lan.get("device").map(str::to_owned).into_iter().collect()
	} else {
		ports
	};
	let bridge = ensure_bridge(ubus, &layout).await?;

	let members: Vec<String> = ports.iter().map(|port| Membership::untagged(port)).collect();
	let vlan = json!({ "device": bridge, "vlan": PRIMARY_VLAN.to_string(), "ports": members });
	uci::add(ubus, "network", "bridge-vlan", Some(&format!("cw_vlan_{PRIMARY}")), vlan).await?;
	uci::set(
		ubus,
		"network",
		PRIMARY,
		json!({ "device": format!("{bridge}.{PRIMARY_VLAN}") }),
	)
	.await?;
	if layout.address(PRIMARY).is_none_or(|address| address == STOCK_ADDRESS) {
		let address = default_address(ubus).await;
		edit::move_reservations(ubus, STOCK_ADDRESS, address).await?;
		uci::unset(ubus, "network", PRIMARY, &["ipaddr", "netmask"]).await?;
		uci::set(ubus, "network", PRIMARY, json!({ "ipaddr": [address.to_string()] })).await?;
	}

	retire_stock_firewall(ubus).await?;
	let network = json!({ "name": "Home", "color": "green", "trusted": "1", "internet": "1" });
	uci::add(ubus, "cowrt", "network", Some(PRIMARY), network).await?;
	firewall::sync(ubus).await?;
	Ok(())
}

pub async fn adopt(ubus: &Ubus) -> Result<(), ubus::Error> {
	if uci::sections_of(ubus, "cowrt", "network")
		.await?
		.iter()
		.any(|s| s.name == PRIMARY)
	{
		return Ok(());
	}
	if let Err(err) = convert(ubus).await {
		for config in CONFIGS {
			let _ = ubus.call("uci", "revert", json!({ "config": config })).await;
		}
		return Err(err);
	}
	for config in CONFIGS {
		ubus.call("uci", "commit", json!({ "config": config })).await?;
	}
	config::reload(ubus, CONFIGS.into_iter()).await;
	eprintln!("cowrtd: set up network layout");
	Ok(())
}
