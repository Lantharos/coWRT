use std::time::{Duration, Instant};

const DECREASE_GAP: Duration = Duration::from_millis(300);
const CALM_BEFORE_INCREASE: Duration = Duration::from_secs(2);
const BUSY: f64 = 0.75;
const IDLE_SHARE: f64 = 0.02;
const SMOOTHING: f64 = 0.35;
const CUT: f64 = 0.9;
const DEEPEST_CUT: f64 = 0.5;
const GROW: f64 = 1.03;
const DRIFT: f64 = 0.05;
const SHARE_OF_BUSIEST: f64 = 0.6;
const FLOOR_SHARE: f64 = 0.02;

struct Direction {
	max: f64,
	min: f64,
	base: f64,
	rate: f64,
	achieved: f64,
}

impl Direction {
	fn new(max_kbit: u32) -> Self {
		let max = f64::from(max_kbit);
		Self {
			max,
			min: max * FLOOR_SHARE,
			base: max * 0.6,
			rate: max * 0.6,
			achieved: 0.0,
		}
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
		self.base = (self.base * 0.8 + self.rate * 0.2).clamp(self.min, self.max);
	}

	fn settle(&mut self, calm: bool) {
		let load = self.achieved / self.rate;
		if calm && load > BUSY {
			self.rate = (self.rate * GROW).min(self.max);
			self.base = self.base.max(self.rate * 0.9).min(self.max);
		} else if self.utilization() < IDLE_SHARE {
			self.rate += (self.base - self.rate) * DRIFT;
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
	pub fn new(download_kbit: u32, upload_kbit: u32) -> Self {
		Self {
			down: Direction::new(download_kbit),
			up: Direction::new(upload_kbit),
			last_cut: None,
			last_bloat: None,
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
