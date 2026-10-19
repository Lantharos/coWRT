use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
const RETRY: Duration = Duration::from_millis(300);
const MIN_BYTES: u64 = 256 * 1024;

#[derive(Clone, Copy)]
pub enum Direction {
	Down,
	Up,
}

pub struct Measured {
	pub rate: u64,
	pub loaded_latency: Option<f32>,
}

#[derive(Clone, Copy)]
pub enum Failure {
	Unreachable,
	Limited,
	Interrupted,
}

impl Failure {
	pub fn message(self) -> &'static str {
		match self {
			Self::Unreachable => "Couldn't reach the speed test servers",
			Self::Limited => "The speed test server is limiting this connection, try again in a bit",
			Self::Interrupted => "The connection dropped during the test",
		}
	}
}

fn refused(err: &ureq::Error) -> bool {
	matches!(err, ureq::Error::StatusCode(403 | 429))
}

#[derive(Default)]
struct Tally {
	bytes: AtomicU64,
	refused: AtomicBool,
}

impl Tally {
	fn failed(&self, err: &ureq::Error) {
		if refused(err) {
			self.refused.store(true, Ordering::Relaxed);
		}
		thread::sleep(RETRY);
	}
}

fn agent() -> Agent {
	Agent::config_builder()
		.timeout_global(Some(Duration::from_secs(20)))
		.user_agent(concat!("coWRT/", env!("CARGO_PKG_VERSION")))
		.build()
		.into()
}

fn ping(agent: &Agent) -> Result<f32, ureq::Error> {
	let start = Instant::now();
	agent.get(format!("{BASE}/__down?bytes=0")).call()?;
	Ok(start.elapsed().as_secs_f32() * 1000.0)
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

pub fn idle_latency(samples: usize) -> Result<f32, Failure> {
	let agent = agent();
	ping(&agent).map_err(|err| {
		if refused(&err) {
			Failure::Limited
		} else {
			Failure::Unreachable
		}
	})?;
	median((0..samples).filter_map(|_| ping(&agent).ok()).collect()).ok_or(Failure::Unreachable)
}

fn pings_until(deadline: Instant) -> Option<f32> {
	let agent = agent();
	let mut times = Vec::new();
	while Instant::now() < deadline {
		times.extend(ping(&agent).ok());
		thread::sleep(PING_GAP);
	}
	median(times)
}

fn download(tally: &Tally, deadline: Instant) {
	let agent = agent();
	let mut buffer = vec![0; CHUNK];
	while Instant::now() < deadline {
		let mut response = match agent.get(format!("{BASE}/__down?bytes={DOWNLOAD_BYTES}")).call() {
			Ok(response) => response,
			Err(err) => {
				tally.failed(&err);
				continue;
			}
		};
		let mut reader = response.body_mut().as_reader();
		while Instant::now() < deadline {
			match reader.read(&mut buffer) {
				Ok(0) | Err(_) => break,
				Ok(read) => tally.bytes.fetch_add(read as u64, Ordering::Relaxed),
			};
		}
	}
}

struct Payload<'a> {
	counter: &'a AtomicU64,
	deadline: Instant,
	left: u64,
	sent: u64,
}

impl Read for Payload<'_> {
	fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
		if self.left == 0 || Instant::now() >= self.deadline {
			return Ok(0);
		}
		let size = buffer.len().min(CHUNK).min(self.left as usize);
		buffer[..size].fill(0);
		self.left -= size as u64;
		self.sent += size as u64;
		self.counter.fetch_add(size as u64, Ordering::Relaxed);
		Ok(size)
	}
}

fn upload(tally: &Tally, deadline: Instant) {
	let agent = agent();
	while Instant::now() < deadline {
		let mut payload = Payload {
			counter: &tally.bytes,
			deadline,
			left: UPLOAD_BYTES,
			sent: 0,
		};
		let sent = agent
			.post(format!("{BASE}/__up"))
			.content_type("application/octet-stream")
			.send(SendBody::from_reader(&mut payload));
		if let Err(err) = sent {
			tally.bytes.fetch_sub(payload.sent, Ordering::Relaxed);
			tally.failed(&err);
		}
	}
}

pub fn transfer(direction: Direction, mut report: impl FnMut(u64)) -> Result<Measured, Failure> {
	let tally = Arc::new(Tally::default());
	let start = Instant::now();
	let deadline = start + PHASE;
	let workers: Vec<_> = (0..STREAMS)
		.map(|_| {
			let tally = tally.clone();
			thread::spawn(move || match direction {
				Direction::Down => download(&tally, deadline),
				Direction::Up => upload(&tally, deadline),
			})
		})
		.collect();
	let pinger = thread::spawn(move || pings_until(deadline));

	let mut last = (start, 0);
	let mut settled = None;
	while Instant::now() < deadline {
		thread::sleep(TICK);
		let now = (Instant::now(), tally.bytes.load(Ordering::Relaxed));
		report(rate(now.1.saturating_sub(last.1), now.0 - last.0));
		if settled.is_none() && now.0 - start >= WARMUP {
			settled = Some(now);
		}
		last = now;
	}
	for worker in workers {
		let _ = worker.join();
	}
	let loaded_latency = pinger.join().ok().flatten();
	let total = tally.bytes.load(Ordering::Relaxed);
	if tally.refused.load(Ordering::Relaxed) {
		return Err(Failure::Limited);
	}
	if total < MIN_BYTES {
		return Err(Failure::Interrupted);
	}
	let (from, base) = settled.unwrap_or((start, 0));
	Ok(Measured {
		rate: rate(total.min(last.1).saturating_sub(base), last.0 - from),
		loaded_latency,
	})
}
