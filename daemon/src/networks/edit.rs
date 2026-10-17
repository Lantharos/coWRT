use serde::Deserialize;
use serde_json::json;

use super::address::Cidr;
use super::firewall;
use super::layout::{Layout, Membership, PRIMARY};
use super::{Dhcp, ports};
use crate::config::uci;
use crate::http::ApiError;
use crate::ubus::{self, Ubus};

pub const COLORS: [&str; 6] = ["green", "blue", "purple", "orange", "pink", "teal"];
const RESERVED: [&str; 5] = ["wan", "wan6", "loopback", "globals", "lan"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInput {
	pub name: String,
	pub color: String,
	pub vlan: u16,
	pub address: String,
	pub dhcp: Dhcp,
	pub trusted: bool,
	pub internet: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInput {
	pub network: Option<String>,
	pub tagged: Vec<String>,
}

fn flag(on: bool) -> &'static str {
	if on { "1" } else { "0" }
}

fn validate(layout: &Layout, id: Option<&str>, input: &NetworkInput) -> Result<Cidr, ApiError> {
	let name = input.name.trim();
	if name.is_empty() || name.len() > 32 {
		return Err(ApiError::bad_request("Names need 1 to 32 characters"));
	}
	if !COLORS.contains(&input.color.as_str()) {
		return Err(ApiError::bad_request("Pick one of the available colors"));
	}
	if !(1..=4094).contains(&input.vlan) {
		return Err(ApiError::bad_request("VLAN numbers go from 1 to 4094"));
	}
	let address: Cidr = input
		.address
		.parse()
		.map_err(|()| ApiError::bad_request("Addresses look like 10.0.10.1/24"))?;
	if !address.is_usable_host() {
		return Err(ApiError::bad_request(
			"Use a host address with a /8 to /30 prefix, like 10.0.10.1/24",
		));
	}
	for other in layout.managed().filter(|n| Some(n.name.as_str()) != id) {
		let other_name = other.get("name").unwrap_or(&other.name);
		if layout.vlan_of(&other.name) == Some(input.vlan) {
			return Err(ApiError::conflict(format!("{other_name} already uses VLAN {}", input.vlan)));
		}
		if layout.address(&other.name).is_some_and(|cidr| cidr.overlaps(address)) {
			return Err(ApiError::conflict(format!("That range overlaps with {other_name}")));
		}
	}
	let hosts = (!address.mask()).saturating_sub(1);
	if input.dhcp.start == 0 || input.dhcp.limit == 0 || input.dhcp.start + input.dhcp.limit - 1 > hosts {
		return Err(ApiError::bad_request("The DHCP range doesn't fit inside the network"));
	}
	Ok(address)
}

fn slug(layout: &Layout, name: &str) -> String {
	let base: String = name
		.chars()
		.filter(char::is_ascii_alphanumeric)
		.map(|c| c.to_ascii_lowercase())
		.take(10)
		.collect();
	let base = if base.starts_with(|c: char| c.is_ascii_alphabetic()) {
		base
	} else {
		format!("net{base}")
	};
	let base: String = base.chars().take(10).collect();
	let taken = |id: &str| RESERVED.contains(&id) || layout.network.iter().any(|s| s.name == id);
	if !taken(&base) {
		return base;
	}
	(2..)
		.map(|n| format!("{}{n}", &base[..base.len().min(8)]))
		.find(|id| !taken(id))
		.unwrap_or(base)
}

async fn sync_firewall(ubus: &Ubus) -> Result<(), ApiError> {
	Ok(firewall::sync(ubus).await?)
}

pub(super) async fn move_reservations(ubus: &Ubus, from: Cidr, to: Cidr) -> Result<(), ubus::Error> {
	if from.network() == to.network() && from.prefix == to.prefix {
		return Ok(());
	}
	for host in uci::sections_of(ubus, "dhcp", "host").await? {
		let Some(ip) = host.get("ip").and_then(|ip| ip.parse().ok()) else {
			continue;
		};
		if from.contains(ip) {
			uci::set(ubus, "dhcp", &host.name, json!({ "ip": from.translate(ip, to).to_string() })).await?;
		}
	}
	Ok(())
}

async fn write(ubus: &Ubus, layout: &Layout, id: &str, input: &NetworkInput, address: Cidr) -> Result<(), ApiError> {
	if let Some(previous) = layout.address(id) {
		move_reservations(ubus, previous, address).await?;
	}
	let bridge = layout.bridge_name();
	let interface = json!({ "proto": "static", "device": format!("{bridge}.{}", input.vlan), "ipaddr": [address.to_string()], "ip6assign": "60" });
	uci::upsert(ubus, "network", "interface", id, interface).await?;
	uci::unset(ubus, "network", id, &["netmask"]).await?;

	match layout.vlan_of(id).and_then(|vlan| layout.vlan_section(vlan)) {
		Some(section) => uci::set(ubus, "network", &section.name, json!({ "vlan": input.vlan.to_string() })).await?,
		None => {
			let vlan = json!({ "device": bridge, "vlan": input.vlan.to_string() });
			uci::add(ubus, "network", "bridge-vlan", Some(&format!("cw_vlan_{id}")), vlan).await?;
		}
	}

	let dhcp = json!({
		"interface": id,
		"start": input.dhcp.start.to_string(),
		"limit": input.dhcp.limit.to_string(),
		"leasetime": input.dhcp.lease_time,
		"ignore": flag(!input.dhcp.enabled),
		"dhcpv4": "server",
		"dhcpv6": "server",
		"ra": "server",
		"ra_slaac": "1",
		"ra_flags": ["managed-config", "other-config"],
	});
	let dhcp_section = layout.dhcp(id).map_or(id, |section| section.name.as_str());
	uci::upsert(ubus, "dhcp", "dhcp", dhcp_section, dhcp).await?;

	let trusted = input.trusted || id == PRIMARY;
	let meta =
		json!({ "name": input.name.trim(), "color": input.color, "trusted": flag(trusted), "internet": flag(input.internet) });
	uci::upsert(ubus, "cowrt", "network", id, meta).await?;
	sync_firewall(ubus).await
}

pub async fn create(ubus: &Ubus, input: &NetworkInput) -> Result<(), ApiError> {
	let layout = Layout::load(ubus).await?;
	let address = validate(&layout, None, input)?;
	let id = slug(&layout, &input.name);
	write(ubus, &layout, &id, input, address).await
}

pub async fn update(ubus: &Ubus, id: &str, input: &NetworkInput) -> Result<(), ApiError> {
	let layout = Layout::load(ubus).await?;
	if !layout.managed().any(|n| n.name == id) {
		return Err(ApiError::not_found("That network no longer exists"));
	}
	let address = validate(&layout, Some(id), input)?;
	write(ubus, &layout, id, input, address).await
}

pub async fn delete(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	if id == PRIMARY {
		return Err(ApiError::conflict("Your main network can't be removed"));
	}
	let layout = Layout::load(ubus).await?;
	let vlan = layout
		.vlan_of(id)
		.ok_or_else(|| ApiError::not_found("That network no longer exists"))?;
	if let Some(ssid) = layout
		.wireless
		.iter()
		.find(|s| s.kind == "wifi-iface" && s.list("network").iter().any(|n| n == id))
		.and_then(|s| s.get("ssid"))
	{
		return Err(ApiError::conflict(format!("Move {ssid} to another network first")));
	}
	if let Some(section) = layout.vlan_section(vlan) {
		uci::delete(ubus, "network", &section.name).await?;
	}
	uci::delete(ubus, "network", id).await?;
	if let Some(section) = layout.dhcp(id) {
		uci::delete(ubus, "dhcp", &section.name).await?;
	}
	uci::delete(ubus, "cowrt", id).await?;
	let remaining: Vec<String> = layout
		.managed()
		.filter(|n| n.name != id)
		.filter_map(|n| layout.vlan_of(&n.name))
		.flat_map(|v| layout.members(v))
		.map(|m| m.port)
		.collect();
	if let Some(bridge) = layout.bridge() {
		let ports: Vec<String> = layout.bridge_ports().into_iter().filter(|p| remaining.contains(p)).collect();
		set_list(ubus, &bridge.name, "ports", ports).await?;
	}
	sync_firewall(ubus).await
}

async fn set_list(ubus: &Ubus, section: &str, option: &str, values: Vec<String>) -> Result<(), ApiError> {
	if values.is_empty() {
		uci::unset(ubus, "network", section, &[option]).await?;
	} else {
		uci::set(ubus, "network", section, json!({ option: values })).await?;
	}
	Ok(())
}

pub async fn update_port(ubus: &Ubus, port: &str, input: &PortInput) -> Result<(), ApiError> {
	let layout = Layout::load(ubus).await?;
	if !ports::detect(&layout.wan_devices(), &layout.bridge_ports())
		.await
		.iter()
		.any(|p| p == port)
	{
		return Err(ApiError::not_found("That port doesn't exist"));
	}
	let managed: Vec<String> = layout.managed().map(|n| n.name.clone()).collect();
	if let Some(unknown) = input.network.iter().chain(&input.tagged).find(|id| !managed.contains(id)) {
		return Err(ApiError::not_found(format!("There's no network called {unknown}")));
	}

	let mut in_bridge = false;
	for id in &managed {
		let Some(section) = layout.vlan_of(id).and_then(|vlan| layout.vlan_section(vlan)) else {
			continue;
		};
		let mut members: Vec<String> = section
			.list("ports")
			.into_iter()
			.filter(|entry| Membership::parse(entry).port != port)
			.collect();
		if input.network.as_deref() == Some(id.as_str()) {
			members.push(Membership::untagged(port));
		} else if input.tagged.contains(id) {
			members.push(Membership::tagged(port));
		}
		in_bridge |= members.iter().any(|entry| Membership::parse(entry).port == port);
		set_list(ubus, &section.name, "ports", members).await?;
	}

	let bridge = layout
		.bridge()
		.ok_or_else(|| ApiError::conflict("The network bridge is missing"))?;
	let mut ports: Vec<String> = layout.bridge_ports().into_iter().filter(|p| p != port).collect();
	if in_bridge {
		ports.push(port.to_owned());
	}
	set_list(ubus, &bridge.name, "ports", ports).await
}
