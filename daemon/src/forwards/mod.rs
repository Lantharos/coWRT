pub mod reach;
pub mod upnp;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::uci::{self, Section};
use crate::http::ApiError;
use crate::networks::Layout;
use crate::ubus::{self, Ubus};

const PREFIX: &str = "cw_pf_";

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
	Tcp,
	Udp,
	Both,
}

impl Protocol {
	fn uci(self) -> Vec<&'static str> {
		match self {
			Self::Tcp => vec!["tcp"],
			Self::Udp => vec!["udp"],
			Self::Both => vec!["tcp", "udp"],
		}
	}

	fn parse(value: Option<&str>) -> Self {
		match value {
			Some("udp") => Self::Udp,
			Some("both") => Self::Both,
			_ => Self::Tcp,
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardInput {
	pub name: String,
	pub mac: String,
	pub protocol: Protocol,
	pub external: String,
	pub internal: Option<String>,
	pub enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forward {
	pub id: String,
	pub name: String,
	pub mac: String,
	pub protocol: Protocol,
	pub external: String,
	pub internal: Option<String>,
	pub enabled: bool,
	pub address: Option<String>,
}

fn ports(value: &str) -> Option<(u16, u16)> {
	let (start, end) = value.split_once('-').unwrap_or((value, value));
	let (start, end): (u16, u16) = (start.trim().parse().ok()?, end.trim().parse().ok()?);
	(start > 0 && start <= end).then_some((start, end))
}

pub fn first_port(external: &str) -> Option<u16> {
	ports(external).map(|(start, _)| start)
}

impl ForwardInput {
	pub fn validate(&self) -> Result<(), ApiError> {
		if self.name.trim().is_empty() || self.name.len() > 48 {
			return Err(ApiError::bad_request("Give the forward a name"));
		}
		let (start, end) = ports(&self.external).ok_or_else(|| ApiError::bad_request("Ports look like 25565 or 8000-8010"))?;
		if let Some(internal) = self.internal.as_deref().filter(|p| !p.trim().is_empty()) {
			let (inner_start, inner_end) =
				ports(internal).ok_or_else(|| ApiError::bad_request("Ports look like 25565 or 8000-8010"))?;
			if inner_end - inner_start != end - start {
				return Err(ApiError::bad_request("Both port ranges need the same size"));
			}
		}
		crate::devices::normalize_mac(&self.mac).ok_or_else(|| ApiError::bad_request("Pick a device"))?;
		Ok(())
	}
}

fn from_section(section: &Section, address: Option<String>) -> Forward {
	Forward {
		id: section.name.clone(),
		name: section.get("name").unwrap_or(&section.name).to_owned(),
		mac: section.get("mac").unwrap_or_default().to_owned(),
		protocol: Protocol::parse(section.get("proto")),
		external: section.get("external").unwrap_or_default().to_owned(),
		internal: section.get("internal").map(str::to_owned),
		enabled: !section.flag("disabled"),
		address,
	}
}

async fn reservations(ubus: &Ubus) -> Result<Vec<(String, String)>, ubus::Error> {
	Ok(uci::sections_of(ubus, "dhcp", "host")
		.await?
		.iter()
		.filter_map(|host| Some((host.list("mac").first()?.to_ascii_lowercase(), host.get("ip")?.to_owned())))
		.collect())
}

pub async fn list(ubus: &Ubus) -> Result<Vec<Forward>, ubus::Error> {
	let reserved = reservations(ubus).await?;
	Ok(uci::sections_of(ubus, "cowrt", "forward")
		.await?
		.iter()
		.map(|section| {
			let mac = section.get("mac").unwrap_or_default().to_ascii_lowercase();
			let address = reserved.iter().find(|(m, _)| *m == mac).map(|(_, ip)| ip.clone());
			from_section(section, address)
		})
		.collect())
}

pub async fn store(ubus: &Ubus, id: Option<&str>, input: &ForwardInput) -> Result<(), ApiError> {
	input.validate()?;
	let values = json!({
		"name": input.name.trim(),
		"mac": crate::devices::normalize_mac(&input.mac),
		"proto": match input.protocol { Protocol::Tcp => "tcp", Protocol::Udp => "udp", Protocol::Both => "both" },
		"external": input.external.trim(),
		"internal": input.internal.as_deref().map(str::trim).filter(|p| !p.is_empty()),
		"disabled": if input.enabled { "0" } else { "1" },
	});
	match id {
		Some(id) => {
			uci::set(ubus, "cowrt", id, values).await?;
			if input.internal.as_deref().is_none_or(|p| p.trim().is_empty()) {
				uci::unset(ubus, "cowrt", id, &["internal"]).await?;
			}
		}
		None => {
			uci::add(ubus, "cowrt", "forward", None, values).await?;
		}
	}
	Ok(())
}

pub async fn sync(ubus: &Ubus) -> Result<(), ApiError> {
	let forwards = list(ubus).await?;
	let layout = Layout::load(ubus).await?;
	let subnets = layout.subnets();
	for section in uci::sections(ubus, "firewall")
		.await?
		.iter()
		.filter(|s| s.name.starts_with(PREFIX))
	{
		uci::delete(ubus, "firewall", &section.name).await?;
	}
	for (index, forward) in forwards.iter().enumerate().filter(|(_, f)| f.enabled) {
		let Some(address) = forward.address.as_deref() else { continue };
		let Some(zone) = address
			.parse()
			.ok()
			.and_then(|ip| subnets.iter().find(|(_, cidr)| cidr.contains(ip)))
			.map(|(id, _)| id)
		else {
			continue;
		};
		let redirect = json!({
			"name": forward.name,
			"src": "wan",
			"src_dport": forward.external,
			"dest": zone,
			"dest_ip": address,
			"dest_port": forward.internal.clone().unwrap_or_else(|| forward.external.clone()),
			"proto": forward.protocol.uci(),
			"target": "DNAT",
		});
		uci::add(ubus, "firewall", "redirect", Some(&format!("{PREFIX}{index}")), redirect).await?;
	}
	Ok(())
}
