use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::wireguard::{self, PeerStatus};
use crate::access::ddns;
use crate::access::forwards::reach;
use crate::http::ApiError;
use crate::networks::Cidr;
use crate::networks::firewall::{self, VPN_SERVER};
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

const SECTION: &str = "server";
const DEFAULT_PORT: u16 = 51820;
const SUBNET: Cidr = Cidr {
	addr: Ipv4Addr::new(10, 250, 0, 1),
	prefix: 24,
};
const PEER_PREFIX: &str = "cwvpn_peer_";
const LISTEN_RULE: &str = "cwvpn_listen";

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
	Everything,
	Internet,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
	pub id: String,
	pub name: String,
	pub address: String,
	pub status: PeerStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	pub enabled: bool,
	pub port: u16,
	pub access: Access,
	pub endpoint: Option<String>,
	pub peers: Vec<Peer>,
}

#[derive(Deserialize)]
pub struct Input {
	pub enabled: bool,
	pub port: u16,
	pub access: Access,
}

async fn settings(ubus: &Ubus) -> Result<Option<Section>, ubus::Error> {
	Ok(uci::sections_of(ubus, "cowrt", "vpn_server").await?.into_iter().next())
}

async fn peers(ubus: &Ubus) -> Result<Vec<Section>, ubus::Error> {
	uci::sections_of(ubus, "cowrt", "vpn_peer").await
}

pub async fn endpoint(ubus: &Ubus) -> Option<String> {
	let ddns = ddns::load(ubus).await.ok().filter(|d| d.enabled && !d.hostname.is_empty());
	match ddns {
		Some(ddns) => Some(ddns.hostname),
		None => reach::public_ipv4().await.ok(),
	}
}

pub async fn view(ubus: &Ubus) -> Result<View, ubus::Error> {
	let server = settings(ubus).await?;
	let enabled = server.as_ref().is_some_and(|s| s.flag("enabled"));
	let live = if enabled {
		wireguard::status(VPN_SERVER).await
	} else {
		Vec::new()
	};
	let peers = peers(ubus)
		.await?
		.into_iter()
		.map(|section| {
			let public = section.get("public_key").unwrap_or_default();
			Peer {
				id: section.name.clone(),
				name: section.get("name").unwrap_or("Device").to_owned(),
				address: section.get("address").unwrap_or_default().to_owned(),
				status: live
					.iter()
					.find(|(key, _)| key == public)
					.map(|(_, s)| s.clone())
					.unwrap_or_default(),
			}
		})
		.collect();
	Ok(View {
		enabled,
		port: server
			.as_ref()
			.and_then(|s| s.get("port")?.parse().ok())
			.unwrap_or(DEFAULT_PORT),
		access: if server.as_ref().and_then(|s| s.get("access")) == Some("internet") {
			Access::Internet
		} else {
			Access::Everything
		},
		endpoint: if enabled { endpoint(ubus).await } else { None },
		peers,
	})
}

async fn write(ubus: &Ubus) -> Result<(), ApiError> {
	let server = settings(ubus).await?;
	let enabled = server.as_ref().is_some_and(|s| s.flag("enabled"));
	for section in uci::sections(ubus, "network")
		.await?
		.iter()
		.filter(|s| s.name.starts_with(PEER_PREFIX))
	{
		uci::delete(ubus, "network", &section.name).await?;
	}
	let _ = uci::delete(ubus, "firewall", LISTEN_RULE).await;
	if !enabled {
		let _ = uci::delete(ubus, "network", VPN_SERVER).await;
		return Ok(firewall::sync(ubus).await?);
	}
	let server = server.expect("enabled implies a section");
	let port = server.get("port").unwrap_or("51820").to_owned();
	let interface = json!({
		"proto": "wireguard",
		"private_key": server.get("private_key"),
		"listen_port": port,
		"addresses": [SUBNET.to_string()],
	});
	uci::upsert(ubus, "network", "interface", VPN_SERVER, interface).await?;
	for (index, peer) in peers(ubus).await?.iter().enumerate() {
		let values = json!({
			"description": peer.get("name"),
			"public_key": peer.get("public_key"),
			"allowed_ips": [format!("{}/32", peer.get("address").unwrap_or_default())],
		});
		uci::add(
			ubus,
			"network",
			&format!("wireguard_{VPN_SERVER}"),
			Some(&format!("{PEER_PREFIX}{index}")),
			values,
		)
		.await?;
	}
	let rule = json!({ "name": "coWRT VPN", "src": "wan", "proto": "udp", "dest_port": port, "target": "ACCEPT" });
	uci::add(ubus, "firewall", "rule", Some(LISTEN_RULE), rule).await?;
	Ok(firewall::sync(ubus).await?)
}

pub async fn store(ubus: &Ubus, input: &Input) -> Result<(), ApiError> {
	if input.port < 1024 {
		return Err(ApiError::bad_request("Pick a port above 1023"));
	}
	if input.enabled {
		wireguard::ensure_installed(ubus)
			.await
			.map_err(|err| ApiError::conflict(err.to_string()))?;
	}
	let existing = settings(ubus).await?;
	let mut values = json!({
		"enabled": if input.enabled { "1" } else { "0" },
		"port": input.port.to_string(),
		"access": match input.access { Access::Everything => "everything", Access::Internet => "internet" },
	});
	if existing.as_ref().and_then(|s| s.get("private_key")).is_none() && input.enabled {
		let (private, _) = wireguard::keypair().await?;
		values["private_key"] = json!(private);
	}
	uci::upsert(ubus, "cowrt", "vpn_server", SECTION, values).await?;
	write(ubus).await
}

fn next_address(taken: &[String]) -> Option<Ipv4Addr> {
	(2..255u32)
		.map(|host| Ipv4Addr::from(SUBNET.network() | host))
		.find(|ip| !taken.contains(&ip.to_string()))
}

pub async fn add_peer(ubus: &Ubus, name: &str) -> Result<(), ApiError> {
	let name = name.trim();
	if name.is_empty() || name.len() > 32 {
		return Err(ApiError::bad_request("Give the device a name"));
	}
	let taken: Vec<String> = peers(ubus)
		.await?
		.iter()
		.filter_map(|p| p.get("address").map(str::to_owned))
		.collect();
	let address = next_address(&taken).ok_or_else(|| ApiError::conflict("The VPN is full"))?;
	let (private, public) = wireguard::keypair().await?;
	let values = json!({ "name": name, "private_key": private, "public_key": public, "address": address.to_string() });
	uci::add(ubus, "cowrt", "vpn_peer", None, values).await?;
	write(ubus).await
}

pub async fn remove_peer(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	uci::delete(ubus, "cowrt", id).await?;
	write(ubus).await
}

pub async fn peer_config(ubus: &Ubus, id: &str, everything: bool) -> Result<String, ApiError> {
	let server = settings(ubus)
		.await?
		.ok_or_else(|| ApiError::conflict("The VPN isn't set up"))?;
	let peer = peers(ubus)
		.await?
		.into_iter()
		.find(|p| p.name == id)
		.ok_or_else(|| ApiError::not_found("That device isn't on the VPN"))?;
	let server_public = wireguard::public_key(server.get("private_key").unwrap_or_default()).await?;
	let endpoint = endpoint(ubus)
		.await
		.ok_or_else(|| ApiError::conflict("Your public address isn't known yet"))?;
	let allowed = if everything {
		"0.0.0.0/0, ::/0".to_owned()
	} else {
		home_ranges(ubus).await?.join(", ")
	};
	Ok(format!(
		"[Interface]\nPrivateKey = {}\nAddress = {}/32\nDNS = {}\n\n[Peer]\nPublicKey = {server_public}\nEndpoint = {endpoint}:{}\nAllowedIPs = {allowed}\nPersistentKeepalive = 25\n",
		peer.get("private_key").unwrap_or_default(),
		peer.get("address").unwrap_or_default(),
		SUBNET.addr,
		server.get("port").unwrap_or("51820"),
	))
}

async fn home_ranges(ubus: &Ubus) -> Result<Vec<String>, ubus::Error> {
	let layout = crate::networks::Layout::load(ubus).await?;
	let mut ranges: Vec<String> = layout
		.subnets()
		.iter()
		.map(|(_, cidr)| format!("{}/{}", Ipv4Addr::from(cidr.network()), cidr.prefix))
		.collect();
	ranges.push(format!("{}/{}", Ipv4Addr::from(SUBNET.network()), SUBNET.prefix));
	Ok(ranges)
}
