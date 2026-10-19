mod config;
mod decide;
mod health;
mod routing;
pub mod tether;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::future::join_all;
use serde::Serialize;
use serde_json::json;
use tokio::sync::Notify;

use config::{Configured, PRIMARY, configured, import};
pub use config::{Kind, Settings, add, forget, retire_mwan3, settings, store_settings};
use decide::{Live, choose, live};
use health::Tracker;
use routing::Route;

use crate::networks::Layout;
use crate::platform::config::uci;
use crate::platform::netlink::conntrack;
use crate::platform::ubus::Ubus;
use crate::state::{AppState, unix_now};

const EVERY: Duration = Duration::from_secs(3);

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
