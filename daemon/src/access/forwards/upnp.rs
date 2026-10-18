use std::path::Path;

use serde::Serialize;
use serde_json::json;
use tokio::process::Command;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::system::packages;

const PACKAGE: &str = "miniupnpd-nftables";
const SERVICE: &str = "/etc/init.d/miniupnpd";
const LEASES: &str = "/var/run/miniupnpd.leases";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
	pub protocol: String,
	pub external: u16,
	pub address: String,
	pub internal: u16,
	pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Upnp {
	pub installed: bool,
	pub enabled: bool,
	pub mappings: Vec<Mapping>,
}

fn parse(line: &str) -> Option<Mapping> {
	let mut fields = line.splitn(6, ':');
	Some(Mapping {
		protocol: fields.next()?.to_owned(),
		external: fields.next()?.parse().ok()?,
		address: fields.next()?.to_owned(),
		internal: fields.next()?.parse().ok()?,
		description: fields.nth(1).unwrap_or_default().to_owned(),
	})
}

pub async fn status(ubus: &Ubus) -> Result<Upnp, ubus::Error> {
	let installed = Path::new(SERVICE).exists();
	let enabled = installed
		&& uci::sections_of(ubus, "upnpd", "upnpd")
			.await?
			.first()
			.is_some_and(|s| s.flag("enabled"));
	let leases = tokio::fs::read_to_string(LEASES).await.unwrap_or_default();
	Ok(Upnp {
		installed,
		enabled,
		mappings: leases.lines().filter_map(parse).collect(),
	})
}

pub async fn set(ubus: &Ubus, enabled: bool, networks: Vec<String>) -> Result<(), ApiError> {
	if enabled {
		packages::ensure(&[PACKAGE])
			.await
			.map_err(|err| ApiError::conflict(err.to_string()))?;
	} else if !Path::new(SERVICE).exists() {
		return Ok(());
	}
	let section = uci::sections_of(ubus, "upnpd", "upnpd")
		.await?
		.first()
		.map(|s| s.name.clone())
		.ok_or_else(|| ApiError::conflict("UPnP has no configuration"))?;
	let values = json!({
		"enabled": if enabled { "1" } else { "0" },
		"enable_upnp": "1",
		"enable_natpmp": "1",
		"internal_iface": networks,
	});
	uci::set(ubus, "upnpd", &section, values).await?;
	ubus.call("uci", "commit", json!({ "config": "upnpd" })).await?;
	let actions: &[&str] = if enabled {
		&["enable", "restart"]
	} else {
		&["stop", "disable"]
	};
	for action in actions {
		let _ = Command::new(SERVICE).arg(action).status().await;
	}
	Ok(())
}
