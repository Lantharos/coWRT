use serde::Serialize;
use serde_json::{Value, json};

use crate::http::ApiError;
use crate::networks::Layout;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};

const DELEGATED_SIZE: &str = "60";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6 {
	enabled: bool,
	address: Option<String>,
	prefix: Option<String>,
}

fn first(status: &Value, key: &str) -> Option<String> {
	let entry = status.get(key)?.as_array()?.first()?;
	Some(format!(
		"{}/{}",
		entry.get("address")?.as_str()?,
		entry.get("mask")?.as_u64()?
	))
}

pub async fn status(ubus: &Ubus) -> Result<Ipv6, ubus::Error> {
	let wan6 = uci::sections_of(ubus, "network", "interface")
		.await?
		.into_iter()
		.find(|s| s.name == "wan6");
	let live = ubus
		.call("network.interface.wan6", "status", json!({}))
		.await
		.unwrap_or_default();
	Ok(Ipv6 {
		enabled: wan6.is_some_and(|s| !s.flag("disabled")),
		address: first(&live, "ipv6-address"),
		prefix: first(&live, "ipv6-prefix"),
	})
}

pub async fn set(ubus: &Ubus, enabled: bool) -> Result<(), ApiError> {
	uci::set(
		ubus,
		"network",
		"wan6",
		json!({ "disabled": if enabled { "0" } else { "1" } }),
	)
	.await?;
	let layout = Layout::load(ubus).await?;
	let service = if enabled { "server" } else { "disabled" };
	for network in layout.managed() {
		let id = &network.name;
		if enabled {
			uci::set(ubus, "network", id, json!({ "ip6assign": DELEGATED_SIZE })).await?;
		} else {
			uci::unset(ubus, "network", id, &["ip6assign"]).await?;
		}
		if let Some(dhcp) = layout.dhcp(id) {
			uci::set(ubus, "dhcp", &dhcp.name, json!({ "ra": service, "dhcpv6": service })).await?;
		}
	}
	Ok(())
}
