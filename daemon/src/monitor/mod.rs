mod connectivity;
mod history;
mod ledger;
mod proc;
mod traffic;
pub mod wan;
mod watcher;

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::watch;

pub use history::History;
use history::Recorder;
use ledger::Ledger;
use proc::{Counters, CpuTimes, Memory};
use traffic::Accounting;
pub use wan::Wan;

use crate::state::{AppState, unix_now};

const ACCOUNTING_SECS: u64 = 2;
const SAVE_SECS: u64 = 3600;
const RECENT_SAMPLES: usize = 45;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
	pub time: u64,
	pub uptime: u64,
	pub cpu: f32,
	pub memory: MemoryUsage,
	pub wan: Wan,
	pub throughput: Throughput,
	pub internet: connectivity::Internet,
	pub usage_today: u64,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct MemoryUsage {
	pub used: u64,
	pub total: u64,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct Throughput {
	pub rx: u64,
	pub tx: u64,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceUsage {
	pub down: u64,
	pub up: u64,
	pub today_down: u64,
	pub today_up: u64,
	pub recent: Vec<u64>,
}

#[derive(Default)]
struct Rate {
	down: u64,
	up: u64,
	recent: VecDeque<u64>,
}

pub struct Monitor {
	latest: watch::Sender<Snapshot>,
	history: Mutex<Recorder>,
	ledger: tokio::sync::Mutex<Ledger>,
	rates: Mutex<HashMap<String, Rate>>,
}

impl Default for Monitor {
	fn default() -> Self {
		Self {
			latest: watch::Sender::new(Snapshot::default()),
			history: Mutex::default(),
			ledger: tokio::sync::Mutex::default(),
			rates: Mutex::default(),
		}
	}
}

impl Monitor {
	pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
		self.latest.subscribe()
	}

	pub fn history(&self) -> History {
		self.history.lock().expect("history lock poisoned").view()
	}

	pub async fn device_usage(&self) -> HashMap<String, DeviceUsage> {
		let ledger = self.ledger.lock().await;
		let rates = self.rates.lock().expect("rates lock poisoned");
		let mut usage: HashMap<String, DeviceUsage> = ledger
			.devices
			.iter()
			.map(|(mac, [down, up])| {
				(
					mac.clone(),
					DeviceUsage {
						today_down: *down,
						today_up: *up,
						..DeviceUsage::default()
					},
				)
			})
			.collect();
		for (mac, rate) in rates.iter() {
			let entry = usage.entry(mac.clone()).or_default();
			entry.down = rate.down;
			entry.up = rate.up;
			entry.recent = rate.recent.iter().copied().collect();
		}
		usage
	}

	pub async fn save(&self) {
		self.ledger.lock().await.save().await;
	}
}

async fn uptime() -> u64 {
	let raw = tokio::fs::read_to_string("/proc/uptime").await.unwrap_or_default();
	raw.split('.').next().and_then(|secs| secs.parse().ok()).unwrap_or_default()
}

async fn account(state: &AppState, accounting: &mut Accounting, wan_device: Option<String>) {
	let gathered = tokio::task::spawn_blocking(|| {
		Ok::<_, std::io::Error>((crate::netlink::conntrack::flows()?, crate::netlink::neighbors::list()?))
	})
	.await;
	let Ok(Ok((flows, neighbors))) = gathered else { return };
	let sample = accounting.sample(flows, &neighbors, wan_device.as_deref());
	let mut ledger = state.monitor.ledger.lock().await;
	let mut rates = state.monitor.rates.lock().expect("rates lock poisoned");
	for rate in rates.values_mut() {
		rate.down = 0;
		rate.up = 0;
	}
	for (mac, (down, up)) in sample.per_device {
		ledger.add_device(&mac, down, up);
		let rate = rates.entry(mac).or_default();
		rate.down = down * 8 / ACCOUNTING_SECS;
		rate.up = up * 8 / ACCOUNTING_SECS;
	}
	rates.retain(|_, rate| {
		if rate.recent.len() == RECENT_SAMPLES {
			rate.recent.pop_front();
		}
		rate.recent.push_back(rate.down + rate.up);
		rate.recent.iter().any(|&bits| bits > 0)
	});
}

pub fn spawn(state: AppState) {
	let internet = connectivity::spawn();
	tokio::spawn(async move {
		*state.monitor.ledger.lock().await = Ledger::load().await;
		let mut ticker = tokio::time::interval(Duration::from_secs(1));
		ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		let mut cpu_before = CpuTimes::read().await.unwrap_or_default();
		let mut wan = Wan::default();
		let mut counters_before: Option<Counters> = None;
		let mut accounting = Accounting::default();
		let mut watcher = watcher::Watcher::default();
		let mut utc_offset = 0i64;

		let mut tick = 0u64;
		loop {
			ticker.tick().await;
			if tick.is_multiple_of(10) {
				wan = Wan::read(&state.ubus).await;
				watcher.check(&state, wan.device.as_deref()).await;
			}
			if tick.is_multiple_of(60) {
				utc_offset = wan::utc_offset(&state.ubus).await.unwrap_or(utc_offset);
			}
			if tick > 0 && tick.is_multiple_of(SAVE_SECS) {
				state.monitor.save().await;
			}

			let now = unix_now();
			let today = (now as i64 + utc_offset) as u64 / 86_400;
			state.monitor.ledger.lock().await.roll_over(today);

			if tick.is_multiple_of(ACCOUNTING_SECS) {
				account(&state, &mut accounting, wan.device.clone()).await;
			}

			let cpu_now = CpuTimes::read().await.unwrap_or(cpu_before);
			let cpu = cpu_now.usage_since(cpu_before);
			cpu_before = cpu_now;

			let memory = Memory::read().await.unwrap_or_default();
			let counters = match &wan.device {
				Some(device) => Counters::read(device).await,
				None => None,
			};
			let delta = match (counters, counters_before) {
				(Some(now), Some(before)) => now.since(before),
				_ => Counters::default(),
			};
			counters_before = counters;
			let usage_today = {
				let mut ledger = state.monitor.ledger.lock().await;
				ledger.total += delta.rx + delta.tx;
				ledger.total
			};

			let snapshot = Snapshot {
				time: now,
				uptime: uptime().await,
				cpu,
				memory: MemoryUsage {
					used: memory.total - memory.available,
					total: memory.total,
				},
				wan: wan.clone(),
				throughput: Throughput {
					rx: delta.rx * 8,
					tx: delta.tx * 8,
				},
				internet: *internet.borrow(),
				usage_today,
			};
			state.monitor.history.lock().expect("history lock poisoned").record(&snapshot);
			state.monitor.latest.send_replace(snapshot);
			tick += 1;
		}
	});
}
