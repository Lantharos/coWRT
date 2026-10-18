mod health;
mod routing;
pub mod tether;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::Notify;

use health::Tracker;
use routing::Route;

use crate::http::ApiError;
use crate::networks::Layout;
use crate::platform::config::uci::{self, Section};
use crate::platform::netlink::conntrack;
use crate::platform::ubus::{self, Ubus};
use crate::state::{AppState, unix_now};

const SETTINGS: &str = "failover";
const PRIMARY: &str = "wan";
const AUTO: &str = "auto";
const EVERY: Duration = Duration::from_secs(3);
const STEADY_SECS: u64 = 60;
const UPLINK_PROTOS: [&str; 6] = ["dhcp", "static", "pppoe", "ncm", "qmi", "mbim"];
const PHONE_HINTS: [&str; 4] = ["phone", "mobile", "usb", "tether"];

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
	Cable,
	Phone,
	Other,
}

impl Kind {
	fn parse(value: Option<&str>) -> Self {
		match value {
			Some("cable") => Self::Cable,
			Some("phone") => Self::Phone,
			_ => Self::Other,
		}
	}

	fn uci(self) -> &'static str {
		match self {
			Self::Cable => "cable",
			Self::Phone => "phone",
			Self::Other => "other",
		}
	}
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Uplink {
	pub id: String,
	pub name: String,
	pub kind: Kind,
	pub device: Option<String>,
	pub connected: bool,
	pub online: bool,
	pub latency_ms: Option<u32>,
	pub in_use: bool,
	pub since: u64,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
	pub mode: String,
	pub failback: bool,
	pub active: Option<String>,
	pub uplinks: Vec<Uplink>,
	pub phone_drivers: bool,
}

#[derive(Deserialize)]
pub struct Settings {
	pub mode: String,
	pub failback: bool,
}

#[derive(Default)]
pub struct Uplinks {
	status: Mutex<Status>,
	wake: Notify,
}

impl Uplinks {
	pub fn status(&self) -> Status {
		self.status.lock().expect("uplinks lock poisoned").clone()
	}

	pub fn active(&self) -> Option<Uplink> {
		let status = self.status.lock().expect("uplinks lock poisoned");
		status.uplinks.iter().find(|u| u.in_use).cloned()
	}

	pub fn active_id(&self) -> String {
		self.active().map_or_else(|| PRIMARY.to_owned(), |u| u.id)
	}

	pub fn nudge(&self) {
		self.wake.notify_one();
	}
}

struct Configured {
	id: String,
	name: String,
	kind: Kind,
}

async fn configured(ubus: &Ubus) -> Result<Vec<Configured>, ubus::Error> {
	Ok(uci::sections_of(ubus, "cowrt", "uplink")
		.await?
		.into_iter()
		.map(|s| Configured {
			name: s.get("name").unwrap_or(&s.name).to_owned(),
			kind: Kind::parse(s.get("kind")),
			id: s.name,
		})
		.collect())
}

pub async fn settings(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", SETTINGS).await?.into_iter().next();
	Ok(Settings {
		mode: section.as_ref().and_then(|s| s.get("mode")).unwrap_or(AUTO).to_owned(),
		failback: section.is_none_or(|s| s.get("failback") != Some("0")),
	})
}

pub async fn store_settings(ubus: &Ubus, input: &Settings) -> Result<(), ubus::Error> {
	let values = json!({ "mode": input.mode, "failback": if input.failback { "1" } else { "0" } });
	uci::upsert(ubus, "cowrt", SETTINGS, SETTINGS, values).await
}

pub async fn add(ubus: &Ubus, id: &str, name: &str, kind: Kind) -> Result<(), ubus::Error> {
	uci::upsert(ubus, "cowrt", "uplink", id, json!({ "name": name, "kind": kind.uci() })).await
}

pub async fn forget(ubus: &Ubus, id: &str) -> Result<(), ubus::Error> {
	uci::delete(ubus, "cowrt", id).await
}

fn masqueraded(firewall: &[Section]) -> Vec<String> {
	firewall
		.iter()
		.filter(|s| s.kind == "zone" && s.flag("masq"))
		.flat_map(|s| s.list("network"))
		.collect()
}

pub async fn retire_mwan3() {
	let script = "/etc/init.d/mwan3";
	if tokio::fs::metadata(script).await.is_err() {
		return;
	}
	for action in ["stop", "disable"] {
		let _ = Command::new(script).arg(action).output().await;
	}
}

async fn import(state: &AppState) -> Result<(), ApiError> {
	if !configured(&state.ubus).await?.is_empty() {
		return Ok(());
	}
	let (network, firewall) = tokio::try_join!(uci::sections(&state.ubus, "network"), uci::sections(&state.ubus, "firewall"))?;
	let outside = masqueraded(&firewall);
	let mut found = vec![(PRIMARY.to_owned(), "Main line".to_owned(), Kind::Cable)];
	for interface in network.iter().filter(|s| s.kind == "interface" && s.name != PRIMARY) {
		let proto = interface.get("proto").unwrap_or_default();
		if !outside.contains(&interface.name) || !UPLINK_PROTOS.contains(&proto) {
			continue;
		}
		let device = interface.get("device").unwrap_or_default();
		let phone = PHONE_HINTS.iter().any(|hint| interface.name.contains(hint)) || tether::is_tether(device).await;
		let (name, kind) = if phone {
			("Phone".to_owned(), Kind::Phone)
		} else {
			(interface.name.clone(), Kind::Other)
		};
		found.push((interface.name.clone(), name, kind));
	}
	let several = found.len() > 1;
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			for (id, name, kind) in &found {
				add(ubus, id, name, *kind).await?;
			}
			Ok(())
		})
		.await?;
	if several {
		retire_mwan3().await;
	}
	Ok(())
}

struct Live {
	connected: bool,
	device: Option<String>,
	gateway: Option<String>,
}

fn live(status: &Value) -> Live {
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

fn choose(settings: &Settings, uplinks: &[Uplink], current: Option<&str>, now: u64) -> Option<String> {
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

async fn forget_lan_flows(ubus: &Ubus) {
	let Ok(layout) = Layout::load(ubus).await else { return };
	let subnets: Vec<_> = layout.subnets().into_iter().map(|(_, cidr)| cidr).collect();
	let _ = tokio::task::spawn_blocking(move || {
		conntrack::forget_where(|flow| match flow.source {
			std::net::IpAddr::V4(ip) => subnets.iter().any(|cidr| cidr.contains(ip)),
			std::net::IpAddr::V6(_) => false,
		})
	})
	.await;
}

async fn follow_phone(state: &AppState, uplink: &Configured, live: &Live) {
	if uplink.kind != Kind::Phone || live.connected {
		return;
	}
	let Some(device) = tether::plugged_in().await else { return };
	let configured = uci::sections(&state.ubus, "network")
		.await
		.ok()
		.and_then(|sections| sections.into_iter().find(|s| s.name == uplink.id))
		.and_then(|s| s.get("device").map(str::to_owned));
	if configured.as_deref() == Some(device.as_str()) {
		return;
	}
	let _ = state
		.applier
		.apply(&state.ubus, &["network"], None, async |ubus| {
			Ok(tether::follow(ubus, &uplink.id, &device).await?)
		})
		.await;
}

async fn tick(state: &AppState, trackers: &mut HashMap<String, Tracker>, steering: &mut bool) {
	let (Ok(uplinks), Ok(settings)) = (configured(&state.ubus).await, settings(&state.ubus).await) else {
		return;
	};
	let now = unix_now();
	let previous = state.uplinks.status().active;
	let mut lives = Vec::with_capacity(uplinks.len());
	for uplink in &uplinks {
		let reply = state
			.ubus
			.call(&format!("network.interface.{}", uplink.id), "status", json!({}))
			.await
			.unwrap_or_default();
		let live = live(&reply);
		follow_phone(state, uplink, &live).await;
		lives.push(live);
	}
	let probes = join_all(lives.iter().map(|live| async move {
		match (&live.device, live.connected) {
			(Some(device), true) => Some(health::probe(device).await),
			_ => None,
		}
	}))
	.await;

	let mut status = Vec::with_capacity(uplinks.len());
	let mut routes = HashMap::new();
	for ((uplink, live), probe) in uplinks.iter().zip(lives).zip(probes) {
		let tracker = trackers.entry(uplink.id.clone()).or_default();
		match probe {
			Some(result) => tracker.record(result, now),
			None => tracker.lost(now),
		}
		if let Some(device) = &live.device {
			routes.insert(uplink.id.clone(), (device.clone(), live.gateway.clone()));
		}
		status.push(Uplink {
			id: uplink.id.clone(),
			name: uplink.name.clone(),
			kind: uplink.kind,
			device: live.device,
			connected: live.connected,
			online: tracker.online,
			latency_ms: tracker.latency_ms,
			in_use: false,
			since: tracker.since,
		});
	}

	let active = if uplinks.len() > 1 {
		choose(&settings, &status, previous.as_deref(), now)
	} else {
		status.first().map(|u| u.id.clone())
	};
	for uplink in &mut status {
		uplink.in_use = active.as_deref() == Some(uplink.id.as_str());
	}

	if uplinks.len() > 1 {
		if let Some((device, gateway)) = active.as_ref().and_then(|id| routes.get(id)) {
			let route = Route {
				device,
				gateway: gateway.as_deref(),
			};
			match routing::steer(&route).await {
				Ok(()) => *steering = true,
				Err(err) => eprintln!("cowrtd: could not route through {device}: {err}"),
			}
		}
		if previous.is_some() && previous != active {
			eprintln!("cowrtd: switched internet to {}", active.as_deref().unwrap_or("nothing"));
			forget_lan_flows(&state.ubus).await;
		}
	} else if *steering {
		routing::release().await;
		*steering = false;
	}

	*state.uplinks.status.lock().expect("uplinks lock poisoned") = Status {
		mode: settings.mode,
		failback: settings.failback,
		active,
		uplinks: status,
		phone_drivers: tether::drivers_installed().await,
	};
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		if let Err(err) = import(&state).await {
			eprintln!("cowrtd: could not look for internet connections: {err}");
		}
		let mut trackers = HashMap::new();
		let mut steering = true;
		loop {
			tick(&state, &mut trackers, &mut steering).await;
			tokio::select! {
				_ = tokio::time::sleep(EVERY) => {}
				_ = state.uplinks.wake.notified() => {}
			}
		}
	});
}
