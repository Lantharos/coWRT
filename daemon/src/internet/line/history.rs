use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

pub const BUCKET_SECS: u64 = 300;
const BUCKETS: usize = 7 * 24 * 3600 / BUCKET_SECS as usize;
const FILE: &str = "/etc/cowrt/line.json";

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
	pub time: u64,
	pub latency: f32,
	pub p95: f32,
	pub loss: f32,
	pub down: u64,
	pub up: u64,
	pub limit_down: Option<u32>,
	pub limit_up: Option<u32>,
}

#[derive(Clone, Default)]
struct Bucket {
	index: u64,
	rtts: Vec<f32>,
	lost: u32,
	down_bytes: u64,
	up_bytes: u64,
	seconds: f64,
	limit_totals: (u64, u64),
	limited: u64,
}

impl Bucket {
	fn point(&mut self) -> Option<Point> {
		let answered = self.rtts.len() as u32;
		let sent = answered + self.lost;
		if sent == 0 || self.seconds == 0.0 {
			return None;
		}
		self.rtts.sort_by(f32::total_cmp);
		let average = |total: u64| (self.limited > 0).then(|| (total / self.limited) as u32);
		Some(Point {
			time: self.index * BUCKET_SECS,
			latency: self.rtts.iter().sum::<f32>() / answered.max(1) as f32,
			p95: self.rtts.get(self.rtts.len() * 95 / 100).copied().unwrap_or_default(),
			loss: self.lost as f32 * 100.0 / sent as f32,
			down: (self.down_bytes as f64 * 8.0 / self.seconds) as u64,
			up: (self.up_bytes as f64 * 8.0 / self.seconds) as u64,
			limit_down: average(self.limit_totals.0),
			limit_up: average(self.limit_totals.1),
		})
	}
}

#[derive(Default)]
pub struct History {
	points: VecDeque<Point>,
	current: Bucket,
}

impl History {
	pub async fn load() -> Self {
		let points: Vec<Point> = tokio::fs::read(FILE)
			.await
			.ok()
			.and_then(|bytes| serde_json::from_slice(&bytes).ok())
			.unwrap_or_default();
		Self {
			points: points.into(),
			current: Bucket::default(),
		}
	}

	pub async fn save(&self) {
		let points: Vec<&Point> = self.points.iter().collect();
		if let Ok(bytes) = serde_json::to_vec(&points) {
			let temporary = format!("{FILE}.new");
			if tokio::fs::write(&temporary, bytes).await.is_ok() {
				let _ = tokio::fs::rename(&temporary, FILE).await;
			}
		}
	}

	fn bucket(&mut self, now: u64) -> &mut Bucket {
		let index = now / BUCKET_SECS;
		if self.current.index != index {
			if let Some(point) = self.current.point() {
				if self.points.len() == BUCKETS {
					self.points.pop_front();
				}
				self.points.push_back(point);
			}
			self.current = Bucket {
				index,
				..Bucket::default()
			};
		}
		&mut self.current
	}

	pub fn reply(&mut self, now: u64, rtt_ms: f64) {
		self.bucket(now).rtts.push(rtt_ms as f32);
	}

	pub fn lost(&mut self, now: u64, count: u32) {
		self.bucket(now).lost += count;
	}

	pub fn traffic(&mut self, now: u64, down_bytes: u64, up_bytes: u64, seconds: f64, limits: Option<(u32, u32)>) {
		let bucket = self.bucket(now);
		bucket.down_bytes += down_bytes;
		bucket.up_bytes += up_bytes;
		bucket.seconds += seconds;
		if let Some((down, up)) = limits {
			bucket.limit_totals.0 += u64::from(down);
			bucket.limit_totals.1 += u64::from(up);
			bucket.limited += 1;
		}
	}

	pub fn points(&self) -> Vec<Point> {
		self.points.iter().copied().chain(self.current.clone().point()).collect()
	}
}
