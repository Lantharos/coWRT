mod nft;
mod schedule;

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

pub use schedule::Schedule;

use crate::networks::Layout;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

pub const MARK_MASK: u32 = 0xf0;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
	Low,
	#[default]
	Normal,
	High,
}

impl Priority {
	pub fn mark(self) -> Option<u32> {
		match self {
			Self::Low => Some(0x10),
			Self::Normal => None,
			Self::High => Some(0x40),
		}
	}

	fn parse(value: Option<&str>) -> Self {
		match value {
			Some("low") => Self::Low,
			Some("high") => Self::High,
			_ => Self::Normal,
		}
	}

	pub fn uci(self) -> Option<&'static str> {
		match self {
			Self::Low => Some("low"),
			Self::Normal => None,
			Self::High => Some("high"),
		}
	}
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Controls {
	pub blocked: bool,
	pub schedules: Vec<Schedule>,
	pub limit_down: Option<u32>,
	pub limit_up: Option<u32>,
	pub priority: Priority,
}

impl Controls {
	pub fn from_section(section: &Section) -> Self {
		Self {
			blocked: section.flag("blocked"),
			schedules: section
				.list("schedule")
				.iter()
				.filter_map(|entry| Schedule::parse(entry))
				.collect(),
			limit_down: section.get("limit_down").and_then(|v| v.parse().ok()),
			limit_up: section.get("limit_up").and_then(|v| v.parse().ok()),
			priority: Priority::parse(section.get("priority")),
		}
	}

	fn is_active(&self) -> bool {
		self.blocked
			|| !self.schedules.is_empty()
			|| self.limit_down.is_some()
			|| self.limit_up.is_some()
			|| self.priority != Priority::Normal
	}
}

pub struct DevicePolicy {
	pub mac: String,
	pub controls: Controls,
}

pub async fn load(ubus: &Ubus) -> Result<Vec<DevicePolicy>, ubus::Error> {
	let people = crate::devices::people::list(ubus).await?;
	Ok(uci::sections_of(ubus, "cowrt", "device")
		.await?
		.iter()
		.filter_map(|section| {
			let mut controls = Controls::from_section(section);
			if let Some(person) = section.get("owner").and_then(|id| people.iter().find(|p| p.id == id)) {
				controls.blocked |= person.blocked;
				controls.schedules.extend(person.schedules.iter().cloned());
			}
			let mac = section.get("mac")?.to_ascii_lowercase();
			controls.is_active().then_some(DevicePolicy { mac, controls })
		})
		.collect())
}

pub fn lan_devices(layout: &Layout) -> Vec<String> {
	let bridge = layout.bridge_name();
	layout
		.managed()
		.filter_map(|network| layout.vlan_of(&network.name))
		.map(|vlan| format!("{bridge}.{vlan}"))
		.collect()
}

pub async fn apply(ubus: &Ubus) -> std::io::Result<()> {
	let layout = Layout::load(ubus).await.map_err(std::io::Error::other)?;
	let policies = load(ubus).await.map_err(std::io::Error::other)?;
	let devices: Vec<String> = lan_devices(&layout)
		.into_iter()
		.filter(|device| nft::device_exists(device))
		.collect();
	let redirects = crate::protection::redirects(ubus).await.map_err(std::io::Error::other)?;
	nft::run(&nft::render(&policies, &devices, &redirects)).await
}

pub async fn cut_off(mac: &str) {
	let mac = mac.to_owned();
	let _ = tokio::task::spawn_blocking(move || {
		let addresses: Vec<IpAddr> = crate::platform::netlink::neighbors::list()?
			.into_iter()
			.filter(|n| n.mac == mac)
			.map(|n| n.ip)
			.collect();
		crate::platform::netlink::conntrack::forget(&addresses)
	})
	.await;
}
