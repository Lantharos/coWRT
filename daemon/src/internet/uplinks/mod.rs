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
use serde_json::{Value, json};
use tokio::sync::Notify;

use config::{Configured, PRIMARY, configured, import};
pub use config::{Kind, Settings, add, forget, retire_mwan3, settings, store_settings};
use decide::{Live, choose, live, live6};
use health::Tracker;
use routing::{Family, Route};

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
	pub ipv6: bool,
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
			std::net::IpAddr::V6(ip) => !ip.is_loopback() && !ip.is_unicast_link_local(),
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

async fn pair_phones(state: &AppState) -> Result<(), crate::http::ApiError> {
	for uplink in configured(&state.ubus).await? {
		if uplink.kind != Kind::Phone || tether::has_ipv6(&state.ubus, &uplink.id).await? {
			continue;
		}
		state
			.applier
			.apply(&state.ubus, &["network", "firewall"], None, async |ubus| {
				Ok(tether::add_ipv6(ubus, &uplink.id).await?)
			})
			.await?;
	}
	Ok(())
}

struct Steering {
	ipv4: bool,
	ipv6: Option<String>,
}

async fn steer_ipv6(target: Option<&(String, Option<String>)>, steering: &mut Steering) {
	let Some((device, gateway)) = target else {
		if steering.ipv6.take().is_some() {
			routing::release(Family::V6).await;
			let _ = routing::masquerade_ipv6(None).await;
		}
		return;
	};
	let route = Route {
		device,
		gateway: gateway.as_deref(),
	};
	if let Err(err) = routing::steer(Family::V6, &route).await {
		eprintln!("cowrtd: could not route IPv6 through {device}: {err}");
		return;
	}
	if steering.ipv6.as_deref() != Some(device.as_str()) {
		match routing::masquerade_ipv6(Some(device)).await {
			Ok(()) => steering.ipv6 = Some(device.clone()),
			Err(err) => eprintln!("cowrtd: could not share IPv6 through {device}: {err}"),
		}
	}
}

fn interface<'a>(dump: &'a [Value], name: &str) -> Option<&'a Value> {
	dump.iter()
		.find(|status| status.get("interface").and_then(Value::as_str) == Some(name))
}

async fn tick(state: &AppState, trackers: &mut HashMap<String, Tracker>, steering: &mut Steering) {
	let (Ok(uplinks), Ok(settings)) = (configured(&state.ubus).await, settings(&state.ubus).await) else {
		return;
	};
	let now = unix_now();
	let previous = state.uplinks.status().active;
	let reply = state
		.ubus
		.call("network.interface", "dump", json!({}))
		.await
		.unwrap_or_default();
	let dump = reply
		.get("interface")
		.and_then(Value::as_array)
		.map_or(&[][..], Vec::as_slice);
	let mut lives = Vec::with_capacity(uplinks.len());
	let mut routes6 = HashMap::new();
	for uplink in &uplinks {
		let live = interface(dump, &uplink.id).map(live).unwrap_or_default();
		follow_phone(state, uplink, &live).await;
		let ipv6 = tether::ipv6_interfaces(&uplink.id)
			.iter()
			.find_map(|name| interface(dump, name).and_then(live6));
		if let Some(Live {
			device: Some(device),
			gateway,
			..
		}) = ipv6
		{
			routes6.insert(uplink.id.clone(), (device, gateway));
		}
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
			ipv6: routes6.contains_key(&uplink.id),
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
			match routing::steer(Family::V4, &route).await {
				Ok(()) => steering.ipv4 = true,
				Err(err) => eprintln!("cowrtd: could not route through {device}: {err}"),
			}
		}
		let backup = active.as_deref().filter(|id| *id != PRIMARY);
		steer_ipv6(backup.and_then(|id| routes6.get(id)), steering).await;
		if previous.is_some() && previous != active {
			eprintln!("cowrtd: switched internet to {}", active.as_deref().unwrap_or("nothing"));
			forget_lan_flows(&state.ubus).await;
		}
	} else if steering.ipv4 {
		routing::release(Family::V4).await;
		steering.ipv4 = false;
		steer_ipv6(None, steering).await;
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
		if let Err(err) = pair_phones(&state).await {
			eprintln!("cowrtd: could not set up IPv6 for the phone: {err}");
		}
		routing::release(Family::V6).await;
		let _ = routing::masquerade_ipv6(None).await;
		let mut trackers = HashMap::new();
		let mut steering = Steering { ipv4: true, ipv6: None };
		loop {
			tick(&state, &mut trackers, &mut steering).await;
			tokio::select! {
				_ = tokio::time::sleep(EVERY) => {}
				_ = state.uplinks.wake.notified() => {}
			}
		}
	});
}
