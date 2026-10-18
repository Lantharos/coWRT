use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

const STORE: &str = "/etc/cowrt/usage.json";
const DAYS: u64 = 30;

#[derive(Default, Serialize, Deserialize)]
pub struct Day {
	day: u64,
	devices: HashMap<String, [u64; 2]>,
}

#[derive(Serialize)]
pub struct DayUsage {
	pub day: u64,
	pub down: u64,
	pub up: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Ledger {
	pub day: u64,
	pub total: u64,
	pub devices: HashMap<String, [u64; 2]>,
	history: VecDeque<Day>,
}

impl Ledger {
	pub async fn load() -> Self {
		let raw = tokio::fs::read(STORE).await.unwrap_or_default();
		serde_json::from_slice(&raw).unwrap_or_default()
	}

	pub async fn save(&self) {
		let Ok(json) = serde_json::to_vec(self) else { return };
		let _ = tokio::fs::create_dir_all("/etc/cowrt").await;
		let temporary = format!("{STORE}.tmp");
		if tokio::fs::write(&temporary, json).await.is_ok() {
			let _ = tokio::fs::rename(temporary, STORE).await;
		}
	}

	pub fn roll_over(&mut self, day: u64) {
		if self.day == day {
			return;
		}
		if self.day != 0 {
			self.history.push_back(Day {
				day: self.day,
				devices: std::mem::take(&mut self.devices),
			});
			while self.history.len() as u64 >= DAYS {
				self.history.pop_front();
			}
		}
		self.day = day;
		self.total = 0;
		self.devices.clear();
	}

	pub fn days(&self, macs: &[String]) -> Vec<DayUsage> {
		let sum = |devices: &HashMap<String, [u64; 2]>| {
			macs.iter()
				.filter_map(|mac| devices.get(mac))
				.fold([0, 0], |acc, [down, up]| [acc[0] + down, acc[1] + up])
		};
		(self.day.saturating_sub(DAYS - 1)..=self.day)
			.map(|day| {
				let [down, up] = if day == self.day {
					sum(&self.devices)
				} else {
					self.history.iter().find(|d| d.day == day).map_or([0, 0], |d| sum(&d.devices))
				};
				DayUsage { day, down, up }
			})
			.collect()
	}

	pub fn add_device(&mut self, mac: &str, down: u64, up: u64) {
		let entry = self.devices.entry(mac.to_owned()).or_default();
		entry[0] += down;
		entry[1] += up;
	}
}
