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

async fn join_wan_zone(ubus: &Ubus, interface: &str) -> Result<(), ubus::Error> {
	if let Some(zone) = wan_zone(ubus).await? {
		let mut networks = zone.list("network");
		if !networks.iter().any(|n| n == interface) {
			networks.push(interface.to_owned());
			uci::set(ubus, "firewall", &zone.name, json!({ "network": networks })).await?;
		}
	}
	Ok(())
}

pub fn ipv6_interfaces(id: &str) -> [String; 2] {
	[format!("{id}6"), format!("{id}_6")]
}

pub async fn has_ipv6(ubus: &Ubus, id: &str) -> Result<bool, ubus::Error> {
	let names = ipv6_interfaces(id);
	Ok(uci::sections_of(ubus, "network", "interface")
		.await?
		.iter()
		.any(|s| names.contains(&s.name)))
}

pub async fn add_ipv6(ubus: &Ubus, id: &str) -> Result<(), ubus::Error> {
	let [name, _] = ipv6_interfaces(id);
	let interface = json!({ "proto": "dhcpv6", "device": format!("@{id}"), "reqaddress": "try", "reqprefix": "no" });
	uci::upsert(ubus, "network", "interface", &name, interface).await?;
	join_wan_zone(ubus, &name).await
}

pub async fn create(ubus: &Ubus, device: &str) -> Result<(), ApiError> {
	let interface = json!({ "proto": "dhcp", "device": device, "metric": PHONE_METRIC, "peerdns": "1" });
	uci::upsert(ubus, "network", "interface", PHONE, interface).await?;
	join_wan_zone(ubus, PHONE).await?;
	Ok(add_ipv6(ubus, PHONE).await?)
}

pub async fn remove(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	let [ipv6, _] = ipv6_interfaces(id);
	let doomed = [id.to_owned(), ipv6];
	for interface in uci::sections_of(ubus, "network", "interface").await? {
		if doomed.contains(&interface.name) {
			uci::delete(ubus, "network", &interface.name).await?;
		}
	}
	if let Some(zone) = wan_zone(ubus).await? {
		let networks: Vec<String> = zone.list("network").into_iter().filter(|n| !doomed.contains(n)).collect();
		uci::set(ubus, "firewall", &zone.name, json!({ "network": networks })).await?;
	}
	Ok(())
}

pub async fn follow(ubus: &Ubus, id: &str, device: &str) -> Result<(), ubus::Error> {
	uci::set(ubus, "network", id, json!({ "device": device })).await
}
