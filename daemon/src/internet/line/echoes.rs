use std::collections::{HashMap, VecDeque};
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

pub const REFLECTORS: [Ipv4Addr; 4] = [
	Ipv4Addr::new(1, 1, 1, 1),
	Ipv4Addr::new(8, 8, 8, 8),
	Ipv4Addr::new(9, 9, 9, 9),
	Ipv4Addr::new(208, 67, 222, 222),
];
const TIMEOUT: Duration = Duration::from_secs(1);
const BASELINE_FALL: f64 = 0.1;
const BASELINE_RISE: f64 = 0.002;
const DELAYED_MS: f64 = 15.0;
const WINDOW: usize = 6;
const BLOATED_AFTER: usize = 3;

pub struct Reply {
	pub rtt_ms: f64,
}

#[derive(Default)]
pub struct Echoes {
	sequence: u16,
	next: usize,
	pending: HashMap<u16, (usize, Instant)>,
	baselines: [Option<f64>; REFLECTORS.len()],
	recent: VecDeque<bool>,
}

impl Echoes {
	pub fn next_target(&mut self) -> (Ipv4Addr, u16) {
		self.sequence = self.sequence.wrapping_add(1);
		let reflector = self.next;
		self.next = (self.next + 1) % REFLECTORS.len();
		self.pending.insert(self.sequence, (reflector, Instant::now()));
		(REFLECTORS[reflector], self.sequence)
	}

	pub fn answered(&mut self, sequence: u16) -> Option<Reply> {
		let (reflector, sent) = self.pending.remove(&sequence)?;
		let rtt_ms = sent.elapsed().as_secs_f64() * 1000.0;
		let baseline = self.baselines[reflector].get_or_insert(rtt_ms);
		let weight = if rtt_ms < *baseline { BASELINE_FALL } else { BASELINE_RISE };
		*baseline += (rtt_ms - *baseline) * weight;
		let delayed = rtt_ms - *baseline > DELAYED_MS;
		if self.recent.len() == WINDOW {
			self.recent.pop_front();
		}
		self.recent.push_back(delayed);
		Some(Reply { rtt_ms })
	}

	pub fn expire(&mut self) -> u32 {
		let before = self.pending.len();
		self.pending.retain(|_, (_, sent)| sent.elapsed() < TIMEOUT);
		(before - self.pending.len()) as u32
	}

	pub fn bloated(&self) -> bool {
		self.recent.iter().filter(|delayed| **delayed).count() >= BLOATED_AFTER
	}

	pub fn baseline_ms(&self) -> Option<f64> {
		self.baselines.iter().flatten().copied().reduce(f64::min)
	}
}
