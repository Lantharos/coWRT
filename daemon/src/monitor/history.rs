use std::collections::VecDeque;

use serde::Serialize;

use super::Snapshot;

const BUCKET_SECS: u64 = 60;
const BUCKETS: usize = 24 * 60;

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
	pub time: u64,
	pub cpu: f32,
	pub memory: f32,
	pub rx: u64,
	pub tx: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
	pub bucket_secs: u64,
	pub points: Vec<Point>,
}

#[derive(Default)]
struct Accumulator {
	bucket: u64,
	samples: u32,
	cpu: f32,
	memory: f32,
	rx: u64,
	tx: u64,
}

impl Accumulator {
	fn point(&self) -> Option<Point> {
		let n = self.samples;
		(n > 0).then(|| Point {
			time: self.bucket * BUCKET_SECS,
			cpu: self.cpu / n as f32,
			memory: self.memory / n as f32,
			rx: self.rx / n as u64,
			tx: self.tx / n as u64,
		})
	}
}

#[derive(Default)]
pub struct Recorder {
	points: VecDeque<Point>,
	current: Accumulator,
}

impl Recorder {
	pub fn record(&mut self, snapshot: &Snapshot) {
		let bucket = snapshot.time / BUCKET_SECS;
		if bucket != self.current.bucket {
			if let Some(point) = self.current.point() {
				if self.points.len() == BUCKETS {
					self.points.pop_front();
				}
				self.points.push_back(point);
			}
			self.current = Accumulator {
				bucket,
				..Accumulator::default()
			};
		}
		let current = &mut self.current;
		current.samples += 1;
		current.cpu += snapshot.cpu;
		current.memory += snapshot.memory.used as f32 / snapshot.memory.total.max(1) as f32 * 100.0;
		current.rx += snapshot.throughput.rx;
		current.tx += snapshot.throughput.tx;
	}

	pub fn view(&self) -> History {
		let points = self.points.iter().copied().chain(self.current.point()).collect();
		History {
			bucket_secs: BUCKET_SECS,
			points,
		}
	}
}
