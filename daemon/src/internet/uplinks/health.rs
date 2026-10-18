use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use tokio::net::TcpSocket;

const TARGETS: [Ipv4Addr; 2] = [Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(9, 9, 9, 9)];
const PORT: u16 = 443;
const TIMEOUT: Duration = Duration::from_secs(2);
const DOWN_AFTER: u32 = 2;
const UP_AFTER: u32 = 2;

async fn connect(device: &str, target: Ipv4Addr) -> Option<u32> {
	let socket = TcpSocket::new_v4().ok()?;
	socket.bind_device(Some(device.as_bytes())).ok()?;
	let started = Instant::now();
	let address = SocketAddr::new(IpAddr::V4(target), PORT);
	tokio::time::timeout(TIMEOUT, socket.connect(address)).await.ok()?.ok()?;
	Some(started.elapsed().as_millis() as u32)
}

pub async fn probe(device: &str) -> Option<u32> {
	for target in TARGETS {
		if let Some(latency) = connect(device, target).await {
			return Some(latency);
		}
	}
	None
}

#[derive(Default)]
pub struct Tracker {
	pub online: bool,
	pub since: u64,
	pub latency_ms: Option<u32>,
	failures: u32,
	successes: u32,
}

impl Tracker {
	pub fn record(&mut self, result: Option<u32>, now: u64) {
		self.latency_ms = result;
		match result {
			Some(_) => {
				self.failures = 0;
				self.successes += 1;
				if !self.online && self.successes >= UP_AFTER {
					self.online = true;
					self.since = now;
				}
			}
			None => {
				self.successes = 0;
				self.failures += 1;
				if self.online && self.failures >= DOWN_AFTER {
					self.online = false;
					self.since = now;
				}
			}
		}
	}

	pub fn lost(&mut self, now: u64) {
		if self.online {
			self.since = now;
		}
		self.online = false;
		self.latency_ms = None;
		self.successes = 0;
	}
}
