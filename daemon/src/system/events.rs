use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::platform::ubus::Ubus;
use crate::state::unix_now;
use crate::system::{release, updates};

const FILE: &str = "/etc/cowrt/events.json";
const KEEP: usize = 500;
const SAVE_AFTER: Duration = Duration::from_secs(5);
const VERSIONS: &str = "/etc/cowrt/versions.json";
const FRESH_BOOT_SECS: f64 = 180.0;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
	Info,
	Good,
	Warn,
	Bad,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
	pub time: u64,
	pub tone: Tone,
	pub message: String,
}

static EVENTS: LazyLock<Mutex<VecDeque<Event>>> = LazyLock::new(Mutex::default);
static UNSAVED: AtomicBool = AtomicBool::new(false);

pub fn record(tone: Tone, message: impl Into<String>) {
	let message = message.into();
	eprintln!("cowrtd: {message}");
	let mut events = EVENTS.lock().expect("events lock poisoned");
	if events.len() == KEEP {
		events.pop_front();
	}
	events.push_back(Event {
		time: unix_now(),
		tone,
		message,
	});
	UNSAVED.store(true, Ordering::Relaxed);
}

pub fn recent(limit: usize) -> Vec<Event> {
	EVENTS
		.lock()
		.expect("events lock poisoned")
		.iter()
		.rev()
		.take(limit)
		.cloned()
		.collect()
}

pub async fn save() {
	if !UNSAVED.swap(false, Ordering::Relaxed) {
		return;
	}
	let json = serde_json::to_vec(&*EVENTS.lock().expect("events lock poisoned"));
	if let Ok(json) = json {
		let temporary = format!("{FILE}.new");
		if tokio::fs::write(&temporary, json).await.is_ok() {
			let _ = tokio::fs::rename(&temporary, FILE).await;
		}
	}
}

async fn load() {
	let saved: VecDeque<Event> = tokio::fs::read(FILE)
		.await
		.ok()
		.and_then(|bytes| serde_json::from_slice(&bytes).ok())
		.unwrap_or_default();
	let mut events = EVENTS.lock().expect("events lock poisoned");
	let recorded_early = std::mem::replace(&mut *events, saved);
	events.extend(recorded_early);
	while events.len() > KEEP {
		events.pop_front();
	}
}

#[derive(PartialEq, Serialize, Deserialize)]
struct Versions {
	cowrt: String,
	openwrt: Option<String>,
}

async fn note_start(ubus: &Ubus) {
	let uptime: f64 = tokio::fs::read_to_string("/proc/uptime")
		.await
		.ok()
		.and_then(|text| text.split_whitespace().next()?.parse().ok())
		.unwrap_or_default();
	record(
		Tone::Info,
		if uptime < FRESH_BOOT_SECS {
			"Router started"
		} else {
			"coWRT restarted"
		},
	);

	let now = Versions {
		cowrt: release::CURRENT.to_owned(),
		openwrt: updates::openwrt_version(ubus).await,
	};
	let before: Option<Versions> = tokio::fs::read(VERSIONS)
		.await
		.ok()
		.and_then(|bytes| serde_json::from_slice(&bytes).ok());
	if let Some(before) = &before {
		if before.cowrt != now.cowrt {
			record(Tone::Good, format!("coWRT updated from {} to {}", before.cowrt, now.cowrt));
		}
		if let (Some(old), Some(new)) = (&before.openwrt, &now.openwrt)
			&& old != new
		{
			record(Tone::Good, format!("OpenWrt updated from {old} to {new}"));
		}
	}
	if before.as_ref() != Some(&now)
		&& let Ok(json) = serde_json::to_vec(&now)
	{
		let _ = tokio::fs::write(VERSIONS, json).await;
	}
}

pub fn spawn(ubus: Ubus) {
	tokio::spawn(async move {
		load().await;
		note_start(&ubus).await;
		loop {
			tokio::time::sleep(SAVE_AFTER).await;
			save().await;
		}
	});
}
