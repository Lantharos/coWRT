pub mod qos;
pub mod speedtest;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::ApiError;
use crate::networks::Cidr;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
	Dhcp,
	Static,
	Pppoe,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Acceleration {
	Off,
	Software,
	Hardware,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
	pub kind: Kind,
	pub port: String,
	pub vlan: Option<u16>,
	pub address: Option<String>,
	pub gateway: Option<String>,
	pub dns: Vec<String>,
	pub username: Option<String>,
	pub password: Option<String>,
	pub mac: Option<String>,
}

fn split_device(device: &str) -> (String, Option<u16>) {
	match device.rsplit_once('.') {
		Some((base, vlan)) if vlan.parse::<u16>().is_ok() => (base.to_owned(), vlan.parse().ok()),
		_ => (device.to_owned(), None),
	}
}

fn interface<'a>(sections: &'a [Section], id: &str) -> Option<&'a Section> {
	sections.iter().find(|s| s.kind == "interface" && s.name == id)
}

pub async fn connection(ubus: &Ubus) -> Result<Connection, ubus::Error> {
	let sections = uci::sections(ubus, "network").await?;
	let wan = interface(&sections, "wan");
	let (port, vlan) = split_device(wan.and_then(|s| s.get("device")).unwrap_or("wan"));
	let kind = match wan.and_then(|s| s.get("proto")) {
		Some("static") => Kind::Static,
		Some("pppoe") => Kind::Pppoe,
		_ => Kind::Dhcp,
	};
	let get = |option: &str| wan.and_then(|s| s.get(option)).map(str::to_owned);
	Ok(Connection {
		kind,
		port,
		vlan,
		address: wan.and_then(Cidr::from_interface).map(|cidr| cidr.to_string()),
		gateway: get("gateway"),
		dns: wan.map(|s| s.list("dns")).unwrap_or_default(),
		username: get("username"),
		password: get("password"),
		mac: get("macaddr"),
	})
}

fn validate(input: &Connection) -> Result<(), ApiError> {
	if input.port.is_empty() || !input.port.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
		return Err(ApiError::bad_request("Pick the port your modem is plugged into"));
	}
	if input.vlan.is_some_and(|vlan| !(1..=4094).contains(&vlan)) {
		return Err(ApiError::bad_request("VLAN numbers go from 1 to 4094"));
	}
	if let Some(mac) = &input.mac
		&& crate::devices::normalize_mac(mac).is_none()
	{
		return Err(ApiError::bad_request("That doesn't look like a MAC address"));
	}
	match input.kind {
		Kind::Static => {
			let address: Cidr = input
				.address
				.as_deref()
				.and_then(|a| a.parse().ok())
				.ok_or_else(|| ApiError::bad_request("Addresses look like 203.0.113.10/24"))?;
			let gateway = input.gateway.as_deref().and_then(|g| g.parse().ok());
			if !gateway.is_some_and(|gateway| address.contains(gateway)) {
				return Err(ApiError::bad_request("The gateway has to be inside the address range"));
			}
		}
		Kind::Pppoe if input.username.as_deref().is_none_or(str::is_empty) => {
			return Err(ApiError::bad_request("Your provider's PPPoE username is needed"));
		}
		_ => {}
	}
	for server in &input.dns {
		if server.parse::<std::net::IpAddr>().is_err() {
			return Err(ApiError::bad_request(format!("{server} isn't an IP address")));
		}
	}
	Ok(())
}

pub async fn store(ubus: &Ubus, input: &Connection) -> Result<(), ApiError> {
	validate(input)?;
	let device = match input.vlan {
		Some(vlan) => format!("{}.{vlan}", input.port),
		None => input.port.clone(),
	};
	let proto = match input.kind {
		Kind::Dhcp => "dhcp",
		Kind::Static => "static",
		Kind::Pppoe => "pppoe",
	};
	uci::unset(
		ubus,
		"network",
		"wan",
		&[
			"ipaddr", "netmask", "gateway", "dns", "username", "password", "macaddr", "peerdns",
		],
	)
	.await?;
	let mut values = json!({ "device": device, "proto": proto });
	match input.kind {
		Kind::Static => {
			values["ipaddr"] = json!([input.address]);
			values["gateway"] = json!(input.gateway);
		}
		Kind::Pppoe => {
			values["username"] = json!(input.username);
			values["password"] = json!(input.password.clone().unwrap_or_default());
		}
		Kind::Dhcp => {}
	}
	if !input.dns.is_empty() {
		values["dns"] = json!(input.dns);
		values["peerdns"] = json!("0");
	}
	if let Some(mac) = input.mac.as_deref().and_then(crate::devices::normalize_mac) {
		values["macaddr"] = json!(mac);
	}
	uci::set(ubus, "network", "wan", values).await?;
	let wan6_device = if input.kind == Kind::Pppoe {
		"@wan".to_owned()
	} else {
		device
	};
	uci::set(ubus, "network", "wan6", json!({ "device": wan6_device })).await?;
	Ok(())
}

pub async fn acceleration(ubus: &Ubus) -> Result<Acceleration, ubus::Error> {
	let defaults = uci::sections_of(ubus, "firewall", "defaults").await?;
	let defaults = defaults.first();
	Ok(
		match (
			defaults.is_some_and(|d| d.flag("flow_offloading")),
			defaults.is_some_and(|d| d.flag("flow_offloading_hw")),
		) {
			(true, true) => Acceleration::Hardware,
			(true, false) => Acceleration::Software,
			_ => Acceleration::Off,
		},
	)
}

pub async fn set_acceleration(ubus: &Ubus, mode: Acceleration) -> Result<(), ApiError> {
	let defaults = uci::sections_of(ubus, "firewall", "defaults").await?;
	let section = defaults
		.first()
		.ok_or_else(|| ApiError::conflict("The firewall has no defaults section"))?;
	let (software, hardware) = match mode {
		Acceleration::Off => ("0", "0"),
		Acceleration::Software => ("1", "0"),
		Acceleration::Hardware => ("1", "1"),
	};
	uci::set(
		ubus,
		"firewall",
		&section.name,
		json!({ "flow_offloading": software, "flow_offloading_hw": hardware }),
	)
	.await?;
	Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6Settings {
	pub enabled: bool,
	pub prefix: Option<u8>,
}

pub async fn ipv6(ubus: &Ubus) -> Result<Ipv6Settings, ubus::Error> {
	let sections = uci::sections(ubus, "network").await?;
	let wan6 = interface(&sections, "wan6");
	Ok(Ipv6Settings {
		enabled: wan6.is_some_and(|s| !s.flag("disabled")),
		prefix: wan6.and_then(|s| s.get("reqprefix")).and_then(|p| p.parse().ok()),
	})
}

pub async fn set_ipv6(ubus: &Ubus, input: &Ipv6Settings) -> Result<(), ApiError> {
	if input.prefix.is_some_and(|size| !(48..=64).contains(&size)) {
		return Err(ApiError::bad_request("Prefix sizes go from /48 to /64"));
	}
	let values = json!({
		"disabled": if input.enabled { "0" } else { "1" },
		"reqaddress": "try",
		"reqprefix": input.prefix.map_or_else(|| "auto".to_owned(), |size| size.to_string()),
	});
	uci::set(ubus, "network", "wan6", values).await?;
	Ok(())
}
