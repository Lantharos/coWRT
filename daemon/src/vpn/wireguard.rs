use std::io;
use std::process::Stdio;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use std::time::Duration;

use serde_json::json;

use crate::packages;
use crate::ubus::Ubus;

pub const PACKAGES: [&str; 2] = ["kmod-wireguard", "wireguard-tools"];

async fn wg(args: &[&str], input: Option<&str>) -> io::Result<String> {
	let mut child = Command::new("wg")
		.args(args)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;
	if let Some(input) = input {
		child.stdin.take().expect("piped stdin").write_all(input.as_bytes()).await?;
	}
	drop(child.stdin.take());
	let output = child.wait_with_output().await?;
	if !output.status.success() {
		return Err(io::Error::other(String::from_utf8_lossy(&output.stderr).trim().to_owned()));
	}
	Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

async fn netifd_knows_wireguard(ubus: &Ubus) -> bool {
	ubus.call("network", "get_proto_handlers", json!({}))
		.await
		.is_ok_and(|handlers| handlers.get("wireguard").is_some())
}

pub async fn ensure_installed(ubus: &Ubus) -> io::Result<()> {
	packages::ensure(&PACKAGES).await?;
	if netifd_knows_wireguard(ubus).await {
		return Ok(());
	}
	Command::new("/etc/init.d/network").arg("restart").status().await?;
	for _ in 0..20 {
		tokio::time::sleep(Duration::from_millis(500)).await;
		if netifd_knows_wireguard(ubus).await {
			return Ok(());
		}
	}
	Err(io::Error::other("The network service didn't pick up WireGuard"))
}

pub async fn keypair() -> io::Result<(String, String)> {
	let private = wg(&["genkey"], None).await?;
	let public = public_key(&private).await?;
	Ok((private, public))
}

pub async fn public_key(private: &str) -> io::Result<String> {
	wg(&["pubkey"], Some(private)).await
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeerStatus {
	pub endpoint: Option<String>,
	pub handshake: Option<u64>,
	pub received: u64,
	pub sent: u64,
}

pub async fn status(interface: &str) -> Vec<(String, PeerStatus)> {
	let Ok(dump) = wg(&["show", interface, "dump"], None).await else {
		return Vec::new();
	};
	dump.lines()
		.skip(1)
		.filter_map(|line| {
			let fields: Vec<&str> = line.split('\t').collect();
			let handshake = fields.get(4)?.parse::<u64>().ok().filter(|t| *t > 0);
			Some((
				fields[0].to_owned(),
				PeerStatus {
					endpoint: fields.get(2).filter(|e| **e != "(none)").map(|e| (*e).to_owned()),
					handshake,
					received: fields.get(5)?.parse().ok()?,
					sent: fields.get(6)?.parse().ok()?,
				},
			))
		})
		.collect()
}

#[derive(Default)]
pub struct ProviderConfig {
	pub private_key: String,
	pub addresses: Vec<String>,
	pub dns: Vec<String>,
	pub public_key: String,
	pub preshared_key: Option<String>,
	pub endpoint_host: String,
	pub endpoint_port: u16,
}

fn values(value: &str) -> Vec<String> {
	value
		.split(',')
		.map(|v| v.trim().to_owned())
		.filter(|v| !v.is_empty())
		.collect()
}

pub fn parse(text: &str) -> Result<ProviderConfig, String> {
	let mut config = ProviderConfig::default();
	let mut section = "";
	for line in text.lines().map(|line| line.split('#').next().unwrap_or_default().trim()) {
		if line.starts_with('[') {
			section = line;
			continue;
		}
		let Some((key, value)) = line.split_once('=') else { continue };
		let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
		match (section, key.as_str()) {
			("[Interface]", "privatekey") => config.private_key = value.to_owned(),
			("[Interface]", "address") => config.addresses = values(value),
			("[Interface]", "dns") => {
				config.dns = values(value)
					.into_iter()
					.filter(|v| v.parse::<std::net::IpAddr>().is_ok())
					.collect()
			}
			("[Peer]", "publickey") => config.public_key = value.to_owned(),
			("[Peer]", "presharedkey") => config.preshared_key = Some(value.to_owned()),
			("[Peer]", "endpoint") => {
				let (host, port) = value.rsplit_once(':').ok_or("The endpoint needs a port")?;
				config.endpoint_host = host.trim_matches(['[', ']']).to_owned();
				config.endpoint_port = port.parse().map_err(|_| "The endpoint port isn't a number")?;
			}
			_ => {}
		}
	}
	if config.private_key.is_empty()
		|| config.public_key.is_empty()
		|| config.endpoint_host.is_empty()
		|| config.addresses.is_empty()
	{
		return Err("That doesn't look like a complete WireGuard config".into());
	}
	Ok(config)
}
