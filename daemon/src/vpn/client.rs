use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::wireguard::{self, PeerStatus};
use crate::config::uci;
use crate::http::ApiError;
use crate::networks::Layout;
use crate::networks::firewall::{self, TUNNEL};
use crate::ubus::{self, Ubus};

const SECTION: &str = "provider";
const TABLE: &str = "200";
const PREFIX: &str = "cwvpn_";
const PEER: &str = "cwvpn_tunnel_peer";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	pub configured: bool,
	pub enabled: bool,
	pub name: String,
	pub endpoint: Option<String>,
	pub networks: Vec<String>,
	pub status: PeerStatus,
}

#[derive(Deserialize)]
pub struct Input {
	pub enabled: bool,
	pub name: String,
	pub config: Option<String>,
	pub networks: Vec<String>,
}

pub async fn view(ubus: &Ubus) -> Result<View, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", "vpn_client").await?.into_iter().next();
	let enabled = section.as_ref().is_some_and(|s| s.flag("enabled"));
	let peer = uci::sections(ubus, "network").await?.into_iter().find(|s| s.name == PEER);
	let status = if enabled {
		wireguard::status(TUNNEL)
			.await
			.into_iter()
			.next()
			.map(|(_, s)| s)
			.unwrap_or_default()
	} else {
		PeerStatus::default()
	};
	Ok(View {
		configured: peer.is_some(),
		enabled,
		name: section
			.as_ref()
			.and_then(|s| s.get("name"))
			.unwrap_or("VPN provider")
			.to_owned(),
		endpoint: peer.and_then(|p| Some(format!("{}:{}", p.get("endpoint_host")?, p.get("endpoint_port")?))),
		networks: section.map(|s| s.list("networks")).unwrap_or_default(),
		status,
	})
}

async fn import(ubus: &Ubus, text: &str) -> Result<Vec<String>, ApiError> {
	let config = wireguard::parse(text).map_err(ApiError::bad_request)?;
	let interface = json!({
		"proto": "wireguard",
		"private_key": config.private_key,
		"addresses": config.addresses,
		"ip4table": TABLE,
		"ip6table": TABLE,
	});
	uci::upsert(ubus, "network", "interface", TUNNEL, interface).await?;
	let _ = uci::delete(ubus, "network", PEER).await;
	let mut peer = json!({
		"public_key": config.public_key,
		"endpoint_host": config.endpoint_host,
		"endpoint_port": config.endpoint_port.to_string(),
		"allowed_ips": ["0.0.0.0/0", "::/0"],
		"route_allowed_ips": "1",
		"persistent_keepalive": "25",
	});
	if let Some(psk) = config.preshared_key {
		peer["preshared_key"] = json!(psk);
	}
	uci::add(ubus, "network", &format!("wireguard_{TUNNEL}"), Some(PEER), peer).await?;
	Ok(config.dns)
}

async fn route(ubus: &Ubus, enabled: bool, networks: &[String], dns: &[String]) -> Result<(), ApiError> {
	let owned = |name: &str| name.starts_with(&format!("{PREFIX}rule_")) || name == format!("{PREFIX}killswitch");
	for section in uci::sections(ubus, "network").await?.iter().filter(|s| owned(&s.name)) {
		uci::delete(ubus, "network", &section.name).await?;
	}
	let layout = Layout::load(ubus).await?;
	if layout.interface(TUNNEL).is_some() {
		uci::set(
			ubus,
			"network",
			TUNNEL,
			json!({ "disabled": if enabled { "0" } else { "1" } }),
		)
		.await?;
	}
	let tunneled = |id: &str| enabled && networks.iter().any(|n| n == id);
	for (id, cidr) in layout.subnets() {
		if let Some(dhcp) = layout.dhcp(&id) {
			if tunneled(&id) && !dns.is_empty() {
				let options: Vec<String> = dns.iter().filter(|d| d.contains('.')).map(|d| format!("6,{d}")).collect();
				uci::set(ubus, "dhcp", &dhcp.name, json!({ "dhcp_option": options })).await?;
			} else {
				uci::unset(ubus, "dhcp", &dhcp.name, &["dhcp_option"]).await?;
			}
		}
		if tunneled(&id) {
			let source = format!("{}/{}", Ipv4Addr::from(cidr.network()), cidr.prefix);
			let rule = json!({ "src": source, "lookup": TABLE, "priority": "1000" });
			uci::add(ubus, "network", "rule", Some(&format!("{PREFIX}rule_{id}")), rule).await?;
		}
	}
	if enabled && !networks.is_empty() {
		let local = json!({ "lookup": "main", "suppress_prefixlength": "0", "priority": "900" });
		uci::add(ubus, "network", "rule", Some(&format!("{PREFIX}rule_local")), local).await?;
		let killswitch =
			json!({ "interface": "loopback", "target": "0.0.0.0/0", "table": TABLE, "type": "unreachable", "metric": "10000" });
		uci::add(ubus, "network", "route", Some(&format!("{PREFIX}killswitch")), killswitch).await?;
	}
	Ok(())
}

pub async fn store(ubus: &Ubus, input: &Input) -> Result<(), ApiError> {
	let existing = uci::sections_of(ubus, "cowrt", "vpn_client").await?.into_iter().next();
	let mut dns = existing.as_ref().map(|s| s.list("dns")).unwrap_or_default();
	if let Some(text) = input.config.as_deref().filter(|t| !t.trim().is_empty()) {
		wireguard::ensure_installed(ubus)
			.await
			.map_err(|err| ApiError::conflict(err.to_string()))?;
		dns = import(ubus, text).await?;
	} else if existing.is_none() {
		return Err(ApiError::bad_request("Paste the WireGuard config from your VPN provider"));
	}
	let managed: Vec<String> = uci::sections_of(ubus, "cowrt", "network")
		.await?
		.into_iter()
		.map(|s| s.name)
		.collect();
	if let Some(unknown) = input.networks.iter().find(|n| !managed.contains(n)) {
		return Err(ApiError::not_found(format!("There's no network called {unknown}")));
	}
	let values = json!({ "enabled": if input.enabled { "1" } else { "0" }, "name": input.name.trim() });
	uci::upsert(ubus, "cowrt", "vpn_client", SECTION, values).await?;
	uci::unset(ubus, "cowrt", SECTION, &["networks", "dns"]).await?;
	for (option, list) in [("networks", &input.networks), ("dns", &dns)] {
		if !list.is_empty() {
			uci::set(ubus, "cowrt", SECTION, json!({ option: list })).await?;
		}
	}
	route(ubus, input.enabled, &input.networks, &dns).await?;
	Ok(firewall::sync(ubus).await?)
}
