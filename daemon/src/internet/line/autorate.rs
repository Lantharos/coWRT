use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const DECREASE_GAP: Duration = Duration::from_millis(300);
const CALM_BEFORE_INCREASE: Duration = Duration::from_secs(2);
const BUSY: f64 = 0.75;
const IDLE_SHARE: f64 = 0.02;
const SMOOTHING: f64 = 0.35;
const LEARNING: f64 = 0.01;
const START_SHARE: f64 = 0.6;
const CUT: f64 = 0.9;
const DEEPEST_CUT: f64 = 0.5;
const GROW: f64 = 1.03;
const DRIFT: f64 = 0.05;
const SHARE_OF_BUSIEST: f64 = 0.6;
const FLOOR_SHARE: f64 = 0.02;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Learned {
	pub download: u32,
	pub upload: u32,
	pub typical_down: f64,
	pub typical_up: f64,
}

struct Direction {
	max: f64,
	min: f64,
	typical: f64,
	rate: f64,
	achieved: f64,
}

impl Direction {
	fn new(max_kbit: u32, typical: Option<f64>) -> Self {
		let max = f64::from(max_kbit);
		let min = max * FLOOR_SHARE;
		let typical = typical.unwrap_or(max * START_SHARE).clamp(min, max);
		Self {
			max,
			min,
			typical,
			rate: typical,
			achieved: 0.0,
		}
	}

	fn learn(&mut self) {
		self.typical += (self.rate - self.typical) * LEARNING;
	}

	fn observe(&mut self, sample: f64) {
		self.achieved += (sample - self.achieved) * SMOOTHING;
	}

	fn utilization(&self) -> f64 {
		self.achieved / self.max
	}

	fn cut(&mut self) {
		self.rate = (self.achieved.min(self.rate) * CUT)
			.max(self.rate * DEEPEST_CUT)
			.max(self.min);
		self.learn();
	}

	fn settle(&mut self, calm: bool) {
		let load = self.achieved / self.rate;
		if calm && load > BUSY {
			self.rate = (self.rate * GROW).min(self.max);
			self.learn();
		} else if self.utilization() < IDLE_SHARE {
			self.rate += (self.typical - self.rate) * DRIFT;
		}
	}
}

pub struct Autorate {
	down: Direction,
	up: Direction,
	last_cut: Option<Instant>,
	last_bloat: Option<Instant>,
}

impl Autorate {
	pub fn new(download_kbit: u32, upload_kbit: u32, learned: Option<Learned>) -> Self {
		let learned = learned.filter(|l| l.download == download_kbit && l.upload == upload_kbit);
		Self {
			down: Direction::new(download_kbit, learned.map(|l| l.typical_down)),
			up: Direction::new(upload_kbit, learned.map(|l| l.typical_up)),
			last_cut: None,
			last_bloat: None,
		}
	}

	pub fn learned(&self) -> Learned {
		Learned {
			download: self.down.max as u32,
			upload: self.up.max as u32,
			typical_down: self.down.typical,
			typical_up: self.up.typical,
		}
	}

	pub fn limits(&self) -> (u32, u32) {
		(self.down.rate as u32, self.up.rate as u32)
	}

	pub fn step(&mut self, bloated: bool, down_kbit: f64, up_kbit: f64) -> (u32, u32) {
		let now = Instant::now();
		self.down.observe(down_kbit);
		self.up.observe(up_kbit);
		if bloated {
			self.last_bloat = Some(now);
			if self.last_cut.is_none_or(|at| now - at >= DECREASE_GAP) {
				let down = self.down.utilization();
				let up = self.up.utilization();
				let busiest = down.max(up);
				if busiest < IDLE_SHARE {
					return self.limits();
				}
				if down >= busiest * SHARE_OF_BUSIEST {
					self.down.cut();
				}
				if up >= busiest * SHARE_OF_BUSIEST {
					self.up.cut();
				}
				self.last_cut = Some(now);
			}
		} else {
			let calm = self.last_bloat.is_none_or(|at| now - at >= CALM_BEFORE_INCREASE);
			self.down.settle(calm);
			self.up.settle(calm);
		}
		self.limits()
	}
}
