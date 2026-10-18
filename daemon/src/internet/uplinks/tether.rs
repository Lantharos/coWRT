use serde_json::json;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::system::packages;

const DRIVERS: [&str; 4] = ["rndis_host", "cdc_ether", "cdc_ncm", "ipheth"];
const PACKAGES: [&str; 5] = [
	"kmod-usb-net-rndis",
	"kmod-usb-net-cdc-ether",
	"kmod-usb-net-cdc-ncm",
	"kmod-usb-net-ipheth",
	"usbmuxd",
];
pub const PHONE: &str = "phone";
const PHONE_METRIC: &str = "20";

pub async fn driver(device: &str) -> Option<String> {
	let link = tokio::fs::read_link(format!("/sys/class/net/{device}/device/driver"))
		.await
		.ok()?;
	Some(link.file_name()?.to_string_lossy().into_owned())
}

pub async fn is_tether(device: &str) -> bool {
	driver(device).await.is_some_and(|name| DRIVERS.contains(&name.as_str()))
}

pub async fn plugged_in() -> Option<String> {
	let mut entries = tokio::fs::read_dir("/sys/class/net").await.ok()?;
	while let Ok(Some(entry)) = entries.next_entry().await {
		let name = entry.file_name().to_string_lossy().into_owned();
		if is_tether(&name).await {
			return Some(name);
		}
	}
	None
}

pub async fn drivers_installed() -> bool {
	packages::installed(PACKAGES[0]).await && packages::installed(PACKAGES[3]).await
}

pub async fn install_drivers() -> Result<(), ApiError> {
	packages::ensure(&PACKAGES)
		.await
		.map_err(|err| ApiError::conflict(err.to_string()))
}

async fn wan_zone(ubus: &Ubus) -> Result<Option<uci::Section>, ubus::Error> {
	Ok(uci::sections_of(ubus, "firewall", "zone")
		.await?
		.into_iter()
		.find(|zone| zone.list("network").iter().any(|n| n == "wan")))
}

pub async fn create(ubus: &Ubus, device: &str) -> Result<(), ApiError> {
	let interface = json!({ "proto": "dhcp", "device": device, "metric": PHONE_METRIC, "peerdns": "1" });
	uci::upsert(ubus, "network", "interface", PHONE, interface).await?;
	if let Some(zone) = wan_zone(ubus).await? {
		let mut networks = zone.list("network");
		if !networks.iter().any(|n| n == PHONE) {
			networks.push(PHONE.to_owned());
			uci::set(ubus, "firewall", &zone.name, json!({ "network": networks })).await?;
		}
	}
	Ok(())
}

pub async fn remove(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	uci::delete(ubus, "network", id).await?;
	if let Some(zone) = wan_zone(ubus).await? {
		let networks: Vec<String> = zone.list("network").into_iter().filter(|n| n != id).collect();
		uci::set(ubus, "firewall", &zone.name, json!({ "network": networks })).await?;
	}
	Ok(())
}

pub async fn follow(ubus: &Ubus, id: &str, device: &str) -> Result<(), ubus::Error> {
	uci::set(ubus, "network", id, json!({ "device": device })).await
}
