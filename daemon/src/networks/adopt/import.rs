use std::collections::HashSet;

use serde_json::json;

use super::super::layout::{DEFAULT_BRIDGE, Layout, Membership, PRIMARY};
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

const RESERVED: [&str; 4] = [PRIMARY, "wan", "wan6", "loopback"];
const COLORS: [&str; 5] = ["orange", "blue", "purple", "pink", "teal"];
const VLAN_STEP: u16 = 10;

pub struct Existing {
	pub id: String,
	trusted: bool,
	internet: bool,
}

fn zone_of<'a>(firewall: &'a [Section], network: &str) -> Option<&'a Section> {
	firewall
		.iter()
		.find(|s| s.kind == "zone" && s.list("network").iter().any(|n| n == network))
}

fn zone_name(zone: &Section) -> &str {
	zone.get("name").unwrap_or(&zone.name)
}

fn forwards(firewall: &[Section], src: &str, dest: &str) -> bool {
	firewall
		.iter()
		.any(|s| s.kind == "forwarding" && s.get("src") == Some(src) && s.get("dest") == Some(dest))
}

pub fn existing(layout: &Layout, firewall: &[Section]) -> Vec<Existing> {
	let zone = |network: &str| zone_of(firewall, network).map(zone_name).unwrap_or(network).to_owned();
	let lan = zone(PRIMARY);
	let wan = zone("wan");
	let outside: HashSet<String> = firewall
		.iter()
		.filter(|s| s.kind == "zone" && s.flag("masq"))
		.flat_map(|s| s.list("network"))
		.collect();
	layout
		.network
		.iter()
		.filter(|s| s.kind == "interface" && !RESERVED.contains(&s.name.as_str()))
		.filter(|s| s.get("proto") == Some("static") && layout.address(&s.name).is_some())
		.filter(|s| !outside.contains(&s.name))
		.map(|s| {
			let own = zone(&s.name);
			Existing {
				id: s.name.clone(),
				trusted: forwards(firewall, &own, &lan),
				internet: forwards(firewall, &own, &wan),
			}
		})
		.collect()
}

fn title(id: &str) -> String {
	let mut chars = id.chars();
	chars
		.next()
		.map(|first| first.to_uppercase().chain(chars).collect())
		.unwrap_or_default()
}

pub async fn move_into_bridge(ubus: &Ubus, network: &Existing, vlan: u16, index: usize) -> Result<(), ubus::Error> {
	let sections = uci::sections(ubus, "network").await?;
	let bridge = sections
		.iter()
		.find(|s| s.kind == "device" && s.get("name") == Some(DEFAULT_BRIDGE))
		.or_else(|| {
			sections
				.iter()
				.find(|s| s.kind == "device" && s.get("type") == Some("bridge"))
		});
	let Some(bridge) = bridge else { return Ok(()) };
	let bridge_name = bridge.get("name").unwrap_or(DEFAULT_BRIDGE).to_owned();
	let device = sections
		.iter()
		.find(|s| s.kind == "interface" && s.name == network.id)
		.and_then(|s| s.get("device"))
		.map(str::to_owned);
	let own_bridge = sections.iter().find(|s| {
		s.kind == "device" && s.get("type") == Some("bridge") && s.get("name") == device.as_deref() && s.name != bridge.name
	});
	let ports: Vec<String> = match (own_bridge, &device) {
		(Some(own), _) => own.list("ports"),
		(None, Some(device)) if !device.starts_with(&bridge_name) => vec![device.clone()],
		_ => Vec::new(),
	};

	let mut bridge_ports = bridge.list("ports");
	bridge_ports.extend(ports.iter().filter(|p| !bridge.list("ports").contains(p)).cloned());
	uci::set(ubus, "network", &bridge.name, json!({ "ports": bridge_ports })).await?;
	let members: Vec<String> = ports.iter().map(|port| Membership::untagged(port)).collect();
	let entry = json!({ "device": bridge_name, "vlan": vlan.to_string(), "ports": members });
	uci::add(
		ubus,
		"network",
		"bridge-vlan",
		Some(&format!("cw_vlan_{}", network.id)),
		entry,
	)
	.await?;
	uci::set(
		ubus,
		"network",
		&network.id,
		json!({ "device": format!("{bridge_name}.{vlan}") }),
	)
	.await?;
	if let Some(own) = own_bridge {
		uci::delete(ubus, "network", &own.name).await?;
	}

	let meta = json!({
		"name": title(&network.id),
		"color": COLORS[index % COLORS.len()],
		"trusted": if network.trusted { "1" } else { "0" },
		"internet": if network.internet { "1" } else { "0" },
	});
	uci::add(ubus, "cowrt", "network", Some(&network.id), meta).await?;
	Ok(())
}

pub fn vlans(count: usize) -> Vec<u16> {
	(1..=count as u16).map(|n| n * VLAN_STEP).collect()
}

pub async fn retire_zones(ubus: &Ubus, firewall: &[Section], networks: &[&str]) -> Result<(), ubus::Error> {
	let mut retired = HashSet::new();
	let mut deleted = HashSet::new();
	for network in networks {
		let Some(zone) = zone_of(firewall, network) else { continue };
		let name = zone_name(zone).to_owned();
		if !retired.insert(name.clone()) {
			continue;
		}
		let doomed = firewall.iter().filter(|s| {
			s.name == zone.name || (s.kind == "forwarding" && (s.get("src") == Some(&name) || s.get("dest") == Some(&name)))
		});
		for section in doomed {
			if deleted.insert(section.name.clone()) {
				uci::delete(ubus, "firewall", &section.name).await?;
			}
		}
		if name == *network {
			continue;
		}
		for section in firewall.iter().filter(|s| s.kind == "rule" || s.kind == "redirect") {
			for side in ["src", "dest"] {
				if section.get(side) == Some(&name) {
					uci::set(ubus, "firewall", &section.name, json!({ side: network })).await?;
				}
			}
		}
	}
	Ok(())
}
