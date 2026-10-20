mod autorate;
mod echoes;
mod history;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::time::{MissedTickBehavior, interval};

use autorate::Autorate;
use echoes::Echoes;
pub use history::Point;
use history::{BUCKET_SECS, History};

use crate::internet::qos::{self, Mode, Settings};
use crate::monitor::wan::{MAIN, Wan};
use crate::platform::icmp::Pinger;
use crate::state::{AppState, unix_now};

const CONTROL: Duration = Duration::from_millis(250);
const QUIET_PING: Duration = Duration::from_secs(1);
const ADAPTIVE_PING: Duration = Duration::from_millis(100);
const RECHECK: Duration = Duration::from_secs(5);
const REAPPLY: Duration = Duration::from_secs(10);
const SAVE: Duration = Duration::from_secs(3600);
const RETRY: Duration = Duration::from_secs(15);
const SMOOTHING: f64 = 0.2;
const MEANINGFUL_CHANGE: f64 = 0.02;

#[derive(Clone, Copy, Serialize)]
pub struct Limits {
	pub download: u32,
	pub upload: u32,
}

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Now {
	pub latency_ms: Option<f32>,
	pub baseline_ms: Option<f32>,
	pub bloated: bool,
	pub limits: Option<Limits>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	pub now: Now,
	pub bucket_secs: u64,
	pub points: Vec<Point>,
}

#[derive(Default)]
pub struct Line {
	history: tokio::sync::Mutex<History>,
	now: Mutex<Now>,
}

impl Line {
	pub async fn view(&self) -> View {
		let now = *self.now.lock().expect("line lock poisoned");
		View {
			now,
			bucket_secs: BUCKET_SECS,
			points: self.history.lock().await.points(),
		}
	}

	pub async fn save(&self) {
		self.history.lock().await.save().await;
	}

	fn publish(&self, now: Now) {
		*self.now.lock().expect("line lock poisoned") = now;
	}
}

async fn counters(device: &str) -> Option<(u64, u64)> {
	let read = |name: &'static str| async move {
		tokio::fs::read_to_string(format!("/sys/class/net/{device}/statistics/{name}"))
			.await
			.ok()?
			.trim()
			.parse::<u64>()
			.ok()
	};
	Some((read("rx_bytes").await?, read("tx_bytes").await?))
}

fn controller(settings: &Settings) -> Option<Autorate> {
	(settings.shaping() && settings.mode == Mode::Adaptive).then(|| Autorate::new(settings.download, settings.upload))
}

fn worth_applying(next: (u32, u32), applied: Option<(u32, u32)>) -> bool {
	let moved = |a: u32, b: u32| (f64::from(a) - f64::from(b)).abs() > f64::from(b) * MEANINGFUL_CHANGE;
	applied.is_none_or(|(down, up)| moved(next.0, down) || moved(next.1, up))
}

struct Traffic {
	last: (u64, u64),
	at: Instant,
}

impl Traffic {
	fn advance(&mut self, current: (u64, u64)) -> (u64, u64, f64) {
		let seconds = self.at.elapsed().as_secs_f64();
		let delta = (current.0.saturating_sub(self.last.0), current.1.saturating_sub(self.last.1));
		self.last = current;
		self.at = Instant::now();
		(delta.0, delta.1, seconds)
	}
}

async fn watch(state: &AppState, device: &str, pinger: &Pinger) {
	let settings = qos::load(&state.ubus).await.unwrap_or_default();
	let mut autorate = controller(&settings);
	let mut echoes = Echoes::default();
	let mut ping = interval(if autorate.is_some() { ADAPTIVE_PING } else { QUIET_PING });
	let mut control = interval(CONTROL);
	ping.set_missed_tick_behavior(MissedTickBehavior::Skip);
	control.set_missed_tick_behavior(MissedTickBehavior::Skip);
	let Some(start) = counters(device).await else { return };
	let mut traffic = Traffic {
		last: start,
		at: Instant::now(),
	};
	let mut latency: Option<f64> = None;
	let mut applied: Option<(u32, u32)> = None;
	let (mut applied_at, mut checked_at, mut saved_at) = (Instant::now(), Instant::now(), Instant::now());

	loop {
		tokio::select! {
			_ = ping.tick() => {
				let (target, sequence) = echoes.next_target();
				let _ = pinger.send(target, sequence);
			}
			reply = pinger.receive() => {
				let Ok(sequence) = reply else { return };
				if let Some(reply) = echoes.answered(sequence) {
					latency = Some(latency.map_or(reply.rtt_ms, |l| l + (reply.rtt_ms - l) * SMOOTHING));
					state.line.history.lock().await.reply(unix_now(), reply.rtt_ms);
				}
			}
			_ = control.tick() => {
				let now = unix_now();
				let lost = echoes.expire();
				let Some(current) = counters(device).await else { return };
				let (down, up, seconds) = traffic.advance(current);
				let kbit = |bytes: u64| bytes as f64 * 8.0 / 1000.0 / seconds.max(0.001);
				let bloated = echoes.bloated();
				let limits = autorate.as_mut().map(|a| a.step(bloated, kbit(down), kbit(up)));
				if let Some(next) = limits
					&& (worth_applying(next, applied) || applied_at.elapsed() >= REAPPLY)
				{
					applied = qos::set_bandwidth(device, next.0, next.1).await.ok().map(|()| next);
					applied_at = Instant::now();
				}
				{
					let mut history = state.line.history.lock().await;
					if lost > 0 {
						history.lost(now, lost);
					}
					history.traffic(now, down, up, seconds, limits);
				}
				state.line.publish(Now {
					latency_ms: latency.map(|l| l as f32),
					baseline_ms: echoes.baseline_ms().map(|b| b as f32),
					bloated,
					limits: limits.map(|(download, upload)| Limits { download, upload }),
				});
				if saved_at.elapsed() >= SAVE {
					state.line.save().await;
					saved_at = Instant::now();
				}
				if checked_at.elapsed() >= RECHECK {
					checked_at = Instant::now();
					let fresh = qos::load(&state.ubus).await.unwrap_or_default();
					let wan = Wan::read(&state.ubus, MAIN).await;
					let same_settings = fresh.mode == settings.mode
						&& fresh.shaping() == settings.shaping()
						&& fresh.download == settings.download
						&& fresh.upload == settings.upload;
					if !same_settings || wan.device.as_deref() != Some(device) {
						return;
					}
				}
			}
		}
	}
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		*state.line.history.lock().await = History::load().await;
		loop {
			let wan = Wan::read(&state.ubus, MAIN).await;
			let device = wan.device.filter(|_| wan.up);
			match device.as_deref().map(Pinger::open) {
				Some(Ok(pinger)) => watch(&state, device.as_deref().unwrap_or_default(), &pinger).await,
				Some(Err(err)) => eprintln!("cowrtd: could not watch the line: {err}"),
				None => {}
			}
			state.line.publish(Now::default());
			tokio::time::sleep(RETRY).await;
		}
	});
}
