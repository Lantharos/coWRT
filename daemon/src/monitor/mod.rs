mod connectivity;
mod history;
mod proc;
mod wan;

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::watch;

pub use history::History;
use history::Recorder;
use proc::{Counters, CpuTimes, Memory};
pub use wan::Wan;

use crate::state::{AppState, RUNTIME_DIR, unix_now};

const USAGE_STORE: &str = "usage";

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

pub struct Monitor {
	latest: watch::Sender<Snapshot>,
	history: Mutex<Recorder>,
}

impl Default for Monitor {
	fn default() -> Self {
		Self {
			latest: watch::Sender::new(Snapshot::default()),
			history: Mutex::default(),
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
}

async fn saved_usage() -> (u64, u64) {
	let raw = tokio::fs::read_to_string(format!("{RUNTIME_DIR}/{USAGE_STORE}"))
		.await
		.unwrap_or_default();
	let mut fields = raw.split_whitespace().filter_map(|field| field.parse().ok());
	(fields.next().unwrap_or_default(), fields.next().unwrap_or_default())
}

async fn save_usage(day: u64, bytes: u64) {
	let _ = tokio::fs::write(format!("{RUNTIME_DIR}/{USAGE_STORE}"), format!("{day} {bytes}")).await;
}

async fn uptime() -> u64 {
	let raw = tokio::fs::read_to_string("/proc/uptime").await.unwrap_or_default();
	raw.split('.').next().and_then(|secs| secs.parse().ok()).unwrap_or_default()
}

pub fn spawn(state: AppState) {
	let internet = connectivity::spawn();
	tokio::spawn(async move {
		let mut ticker = tokio::time::interval(Duration::from_secs(1));
		ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		let mut cpu_before = CpuTimes::read().await.unwrap_or_default();
		let mut wan = Wan::default();
		let mut counters_before: Option<Counters> = None;
		let (mut day, mut usage_today) = saved_usage().await;
		let mut utc_offset = 0i64;

		let mut tick = 0u64;
		loop {
			ticker.tick().await;
			if tick.is_multiple_of(10) {
				wan = Wan::read(&state.ubus).await;
			}
			if tick.is_multiple_of(60) {
				utc_offset = wan::utc_offset(&state.ubus).await.unwrap_or(utc_offset);
				if tick > 0 {
					save_usage(day, usage_today).await;
				}
			}

			let now = unix_now();
			let today = (now as i64 + utc_offset) as u64 / 86_400;
			if today != day {
				day = today;
				usage_today = 0;
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
			usage_today += delta.rx + delta.tx;

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
