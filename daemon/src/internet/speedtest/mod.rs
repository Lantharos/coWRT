mod measure;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use measure::Direction;

use crate::internet::qos;
use crate::monitor::wan::Wan;
use crate::state::{AppState, unix_now};

const STORE: &str = "/etc/cowrt/speedtests.json";
const KEPT: usize = 30;
const LATENCY_SAMPLES: usize = 10;
const TRACE: usize = 40;
const SHAPED_SHARE: f64 = 0.9;

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
	#[default]
	Latency,
	Download,
	Upload,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
	phase: Phase,
	latency: Option<f32>,
	download: Option<u64>,
	upload: Option<u64>,
	current: u64,
	trace: Vec<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
	time: u64,
	download: u64,
	upload: u64,
	latency: f32,
	loaded_latency: Option<f32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
	running: Option<Progress>,
	error: Option<String>,
	history: Vec<Result>,
}

#[derive(Default)]
pub struct SpeedTest {
	running: Arc<Mutex<Option<Progress>>>,
	error: Mutex<Option<String>>,
}

async fn history() -> Vec<Result> {
	let raw = tokio::fs::read(STORE).await.unwrap_or_default();
	serde_json::from_slice(&raw).unwrap_or_default()
}

async fn remember(result: Result) {
	let mut kept = history().await;
	kept.push(result);
	let excess = kept.len().saturating_sub(KEPT);
	kept.drain(..excess);
	let Ok(json) = serde_json::to_vec(&kept) else { return };
	let temporary = format!("{STORE}.tmp");
	let _ = tokio::fs::create_dir_all("/etc/cowrt").await;
	if tokio::fs::write(&temporary, json).await.is_ok() {
		let _ = tokio::fs::rename(temporary, STORE).await;
	}
}

fn update(progress: &Mutex<Option<Progress>>, change: impl FnOnce(&mut Progress)) {
	if let Some(progress) = progress.lock().expect("speed test lock poisoned").as_mut() {
		change(progress);
	}
}

fn measure(progress: &Mutex<Option<Progress>>) -> Option<Result> {
	let latency = measure::idle_latency(LATENCY_SAMPLES)?;
	update(progress, |p| {
		p.latency = Some(latency);
		p.phase = Phase::Download;
	});
	let live = |current: u64| {
		update(progress, |p| {
			p.current = current;
			if p.trace.len() == TRACE {
				p.trace.remove(0);
			}
			p.trace.push(current);
		})
	};
	let down = measure::transfer(Direction::Down, live);
	update(progress, |p| {
		p.download = Some(down.rate);
		p.phase = Phase::Upload;
		p.trace.clear();
	});
	let up = measure::transfer(Direction::Up, live);
	let loaded_latency = match (down.loaded_latency, up.loaded_latency) {
		(Some(a), Some(b)) => Some(a.max(b)),
		(a, b) => a.or(b),
	};
	Some(Result {
		time: unix_now(),
		download: down.rate,
		upload: up.rate,
		latency,
		loaded_latency,
	})
}

async fn tune(state: &AppState, result: &Result) {
	let Ok(mut settings) = qos::load(&state.ubus).await else {
		return;
	};
	if !settings.enabled || !settings.auto {
		return;
	}
	settings.download = (result.download as f64 * SHAPED_SHARE / 1000.0) as u32;
	settings.upload = (result.upload as f64 * SHAPED_SHARE / 1000.0) as u32;
	let _ = state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(qos::store(ubus, &settings).await?)
		})
		.await;
}

impl SpeedTest {
	pub async fn overview(&self) -> Overview {
		let running = self.running.lock().expect("speed test lock poisoned").clone();
		let error = self.error.lock().expect("speed test lock poisoned").clone();
		Overview {
			running,
			error,
			history: history().await,
		}
	}

	pub fn start(&self, state: AppState) -> bool {
		{
			let mut running = self.running.lock().expect("speed test lock poisoned");
			if running.is_some() {
				return false;
			}
			*running = Some(Progress::default());
		}
		*self.error.lock().expect("speed test lock poisoned") = None;
		let progress = self.running.clone();
		tokio::spawn(async move {
			let wan = Wan::read(&state.ubus).await;
			qos::clear(wan.device.as_deref()).await;
			let measuring = progress.clone();
			let result = tokio::task::spawn_blocking(move || measure(&measuring)).await.ok().flatten();
			match &result {
				Some(result) => {
					tune(&state, result).await;
					remember(result.clone()).await;
				}
				None => {
					*state.speedtest.error.lock().expect("speed test lock poisoned") =
						Some("Couldn't reach the speed test servers".to_owned());
				}
			}
			let _ = qos::apply(&state.ubus, wan.device.as_deref()).await;
			*progress.lock().expect("speed test lock poisoned") = None;
		});
		true
	}
}
