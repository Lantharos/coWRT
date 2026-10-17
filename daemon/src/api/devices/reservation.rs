use std::net::Ipv4Addr;

use serde_json::{Value, json};

use crate::config::uci;
use crate::devices::{self, section_id};
use crate::http::ApiError;
use crate::networks::Layout;
use crate::state::AppState;

fn dns_name(name: &str) -> String {
	let label: String = name
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() {
				c.to_ascii_lowercase()
			} else {
				'-'
			}
		})
		.collect();
	label.trim_matches('-').chars().take(63).collect()
}

async fn validate(state: &AppState, mac: &str, ip: Ipv4Addr) -> Result<(), ApiError> {
	let layout = Layout::load(&state.ubus).await?;
	let inside = layout.subnets().into_iter().find(|(_, cidr)| cidr.contains(ip));
	match inside {
		None => return Err(ApiError::bad_request("Pick an address inside one of your networks")),
		Some((_, cidr)) if cidr.addr == ip || !cidr.with_host(ip).is_usable_host() => {
			return Err(ApiError::bad_request(format!(
				"{ip} belongs to the router or the network itself"
			)));
		}
		Some(_) => {}
	}
	let taken = uci::sections_of(&state.ubus, "dhcp", "host")
		.await?
		.into_iter()
		.any(|host| host.get("ip") == Some(&ip.to_string()) && !host.list("mac").iter().any(|m| m.eq_ignore_ascii_case(mac)));
	if taken {
		return Err(ApiError::conflict("Another device already has that address reserved"));
	}
	Ok(())
}

pub async fn set(state: &AppState, mac: &str, reserved: Option<Ipv4Addr>) -> Result<(), ApiError> {
	if let Some(ip) = reserved {
		validate(state, mac, ip).await?;
	}
	let devices = devices::inventory(&state.ubus, &Default::default(), None).await?;
	let name = devices
		.iter()
		.find(|d| d.mac == mac)
		.map(|d| dns_name(&d.name))
		.filter(|n| !n.is_empty());
	let section = format!("cowrt{}", section_id(mac));
	state
		.applier
		.apply(&state.ubus, &["dhcp"], None, async |ubus| {
			let existing = uci::sections_of(ubus, "dhcp", "host").await?;
			let duplicates = existing
				.iter()
				.filter(|h| h.name != section && h.list("mac").iter().any(|m| m.eq_ignore_ascii_case(mac)));
			for host in duplicates {
				uci::delete(ubus, "dhcp", &host.name).await?;
			}
			match reserved {
				Some(ip) => {
					let mut values = json!({ "mac": mac, "ip": ip.to_string(), "dns": "1" });
					if let Some(name) = &name {
						values["name"] = Value::from(name.as_str());
					}
					uci::upsert(ubus, "dhcp", "host", &section, values).await?;
				}
				None if existing.iter().any(|h| h.name == section) => uci::delete(ubus, "dhcp", &section).await?,
				None => {}
			}
			Ok(())
		})
		.await?;
	Ok(())
}
