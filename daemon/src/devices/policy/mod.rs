mod nft;
mod schedule;

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use serde_json::json;

pub use schedule::Schedule;

use crate::networks::Layout;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

pub const MARK_MASK: u32 = 0xf0;
const MAX_LIMIT_KBIT: u32 = 10_000_000;

pub fn check_limit(kbit: u32) -> Result<(), String> {
	if kbit == 0 || kbit > MAX_LIMIT_KBIT {
		return Err("Speed limits go from 1 Kbps to 10 Gbps".into());
	}
	Ok(())
}

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

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
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

	pub fn validate(&self) -> Result<(), String> {
		for schedule in &self.schedules {
			schedule.validate()?;
		}
		for limit in [self.limit_down, self.limit_up].into_iter().flatten() {
			check_limit(limit)?;
		}
		Ok(())
	}

	pub async fn store(&self, ubus: &Ubus, section: &str) -> Result<(), ubus::Error> {
		uci::unset(ubus, "cowrt", section, &["schedule", "limit_down", "limit_up", "priority"]).await?;
		let mut values = json!({ "blocked": if self.blocked { "1" } else { "0" } });
		if !self.schedules.is_empty() {
			values["schedule"] = json!(self.schedules.iter().map(Schedule::to_uci).collect::<Vec<_>>());
		}
		if let Some(down) = self.limit_down {
			values["limit_down"] = json!(down.to_string());
		}
		if let Some(up) = self.limit_up {
			values["limit_up"] = json!(up.to_string());
		}
		if let Some(priority) = self.priority.uci() {
			values["priority"] = json!(priority);
		}
		uci::set(ubus, "cowrt", section, values).await
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

pub struct SharedLimit {
	pub macs: Vec<String>,
	pub down: Option<u32>,
	pub up: Option<u32>,
}

pub struct Policies {
	pub devices: Vec<DevicePolicy>,
	pub shared: Vec<SharedLimit>,
}

pub async fn load(ubus: &Ubus) -> Result<Policies, ubus::Error> {
	let people = crate::devices::people::list(ubus).await?;
	let sections = uci::sections_of(ubus, "cowrt", "device").await?;
	let devices = sections
		.iter()
		.filter_map(|section| {
			let mut controls = Controls::from_section(section);
			if let Some(person) = section.get("owner").and_then(|id| people.iter().find(|p| p.id == id)) {
				controls.blocked |= person.controls.blocked;
				controls.schedules.extend(person.controls.schedules.iter().cloned());
				if controls.priority == Priority::Normal {
					controls.priority = person.controls.priority;
				}
			}
			let mac = section.get("mac")?.to_ascii_lowercase();
			controls.is_active().then_some(DevicePolicy { mac, controls })
		})
		.collect();
	let shared = people
		.iter()
		.filter(|person| person.controls.limit_down.is_some() || person.controls.limit_up.is_some())
		.map(|person| SharedLimit {
			macs: sections
				.iter()
				.filter(|section| section.get("owner") == Some(person.id.as_str()))
				.filter_map(|section| section.get("mac").map(str::to_ascii_lowercase))
				.collect(),
			down: person.controls.limit_down,
			up: person.controls.limit_up,
		})
		.filter(|limit| !limit.macs.is_empty())
		.collect();
	Ok(Policies { devices, shared })
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
	crate::platform::nft::run(&nft::render(&policies, &devices, &redirects)).await
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
