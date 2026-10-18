use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use ureq::{Agent, SendBody};

const BASE: &str = "https://speed.cloudflare.com";
const STREAMS: usize = 6;
const PHASE: Duration = Duration::from_secs(8);
const WARMUP: Duration = Duration::from_millis(1500);
const TICK: Duration = Duration::from_millis(250);
const PING_GAP: Duration = Duration::from_millis(300);
const CHUNK: usize = 64 * 1024;
const DOWNLOAD_BYTES: u64 = 25_000_000;
const UPLOAD_BYTES: u64 = 25_000_000;

#[derive(Clone, Copy)]
pub enum Direction {
	Down,
	Up,
}

pub struct Measured {
	pub rate: u64,
	pub loaded_latency: Option<f32>,
}

fn agent() -> Agent {
	Agent::config_builder()
		.timeout_global(Some(Duration::from_secs(20)))
		.user_agent(concat!("coWRT/", env!("CARGO_PKG_VERSION")))
		.build()
		.into()
}

fn ping(agent: &Agent) -> Option<f32> {
	let start = Instant::now();
	agent.get(format!("{BASE}/__down?bytes=0")).call().ok()?;
	Some(start.elapsed().as_secs_f32() * 1000.0)
}

fn median(mut values: Vec<f32>) -> Option<f32> {
	if values.is_empty() {
		return None;
	}
	values.sort_by(f32::total_cmp);
	Some(values[values.len() / 2])
}

fn rate(bytes: u64, elapsed: Duration) -> u64 {
	(bytes as f64 * 8.0 / elapsed.as_secs_f64().max(0.001)) as u64
}

pub fn idle_latency(samples: usize) -> Option<f32> {
	let agent = agent();
	ping(&agent)?;
	median((0..samples).filter_map(|_| ping(&agent)).collect())
}

fn pings_until(deadline: Instant) -> Option<f32> {
	let agent = agent();
	let mut times = Vec::new();
	while Instant::now() < deadline {
		times.extend(ping(&agent));
		thread::sleep(PING_GAP);
	}
	median(times)
}

fn download(counter: &AtomicU64, deadline: Instant) {
	let agent = agent();
	let mut buffer = vec![0; CHUNK];
	while Instant::now() < deadline {
		let Ok(mut response) = agent.get(format!("{BASE}/__down?bytes={DOWNLOAD_BYTES}")).call() else {
			return;
		};
		let mut reader = response.body_mut().as_reader();
		while Instant::now() < deadline {
			match reader.read(&mut buffer) {
				Ok(0) | Err(_) => break,
				Ok(read) => counter.fetch_add(read as u64, Ordering::Relaxed),
			};
		}
	}
}

struct Payload<'a> {
	counter: &'a AtomicU64,
	deadline: Instant,
	left: u64,
}

impl Read for Payload<'_> {
	fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
		if self.left == 0 || Instant::now() >= self.deadline {
			return Ok(0);
		}
		let size = buffer.len().min(CHUNK).min(self.left as usize);
		buffer[..size].fill(0);
		self.left -= size as u64;
		self.counter.fetch_add(size as u64, Ordering::Relaxed);
		Ok(size)
	}
}

fn upload(counter: &AtomicU64, deadline: Instant) {
	let agent = agent();
	while Instant::now() < deadline {
		let mut payload = Payload {
			counter,
			deadline,
			left: UPLOAD_BYTES,
		};
		let sent = agent
			.post(format!("{BASE}/__up"))
			.content_type("application/octet-stream")
			.send(SendBody::from_reader(&mut payload));
		if sent.is_err() {
			return;
		}
	}
}

pub fn transfer(direction: Direction, mut report: impl FnMut(u64)) -> Measured {
	let counter = Arc::new(AtomicU64::new(0));
	let start = Instant::now();
	let deadline = start + PHASE;
	let workers: Vec<_> = (0..STREAMS)
		.map(|_| {
			let counter = counter.clone();
			thread::spawn(move || match direction {
				Direction::Down => download(&counter, deadline),
				Direction::Up => upload(&counter, deadline),
			})
		})
		.collect();
	let pinger = thread::spawn(move || pings_until(deadline));

	let mut last = (start, 0);
	let mut settled = None;
	while Instant::now() < deadline {
		thread::sleep(TICK);
		let now = (Instant::now(), counter.load(Ordering::Relaxed));
		report(rate(now.1 - last.1, now.0 - last.0));
		if settled.is_none() && now.0 - start >= WARMUP {
			settled = Some(now);
		}
		last = now;
	}
	let (from, base) = settled.unwrap_or((start, 0));
	let measured = rate(last.1 - base, last.0 - from);
	for worker in workers {
		let _ = worker.join();
	}
	Measured {
		rate: measured,
		loaded_latency: pinger.join().ok().flatten(),
	}
}
