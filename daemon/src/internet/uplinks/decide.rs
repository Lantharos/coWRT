use serde_json::Value;

use super::Uplink;
use super::config::{AUTO, Settings};

const STEADY_SECS: u64 = 60;

pub struct Live {
	pub connected: bool,
	pub device: Option<String>,
	pub gateway: Option<String>,
}

pub fn live(status: &Value) -> Live {
	let gateway = status.get("route").and_then(Value::as_array).and_then(|routes| {
		routes
			.iter()
			.find(|route| route.get("target").and_then(Value::as_str) == Some("0.0.0.0"))
			.and_then(|route| route.get("nexthop")?.as_str())
			.filter(|hop| *hop != "0.0.0.0")
			.map(str::to_owned)
	});
	Live {
		connected: status.get("up").and_then(Value::as_bool).unwrap_or_default(),
		device: status.get("l3_device").and_then(Value::as_str).map(str::to_owned),
		gateway,
	}
}

pub fn choose(settings: &Settings, uplinks: &[Uplink], current: Option<&str>, now: u64) -> Option<String> {
	let usable = |u: &&Uplink| u.connected && u.online;
	if settings.mode != AUTO
		&& let Some(pinned) = uplinks.iter().filter(usable).find(|u| u.id == settings.mode)
	{
		return Some(pinned.id.clone());
	}
	let best = uplinks.iter().find(usable);
	let current_usable = current.and_then(|id| uplinks.iter().filter(usable).find(|u| u.id == id));
	match (best, current_usable) {
		(Some(best), Some(current)) if best.id != current.id => {
			let steady = now.saturating_sub(best.since) >= STEADY_SECS;
			Some(if settings.failback && steady { &best.id } else { &current.id }.clone())
		}
		(Some(best), _) => Some(best.id.clone()),
		(None, _) => current
			.map(str::to_owned)
			.or_else(|| uplinks.iter().find(|u| u.connected).map(|u| u.id.clone())),
	}
}
