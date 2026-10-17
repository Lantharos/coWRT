use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const STORE: &str = "/etc/cowrt/usage.json";

#[derive(Default, Serialize, Deserialize)]
pub struct Ledger {
	pub day: u64,
	pub total: u64,
	pub devices: HashMap<String, [u64; 2]>,
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
		if self.day != day {
			*self = Self { day, ..Self::default() };
		}
	}

	pub fn add_device(&mut self, mac: &str, down: u64, up: u64) {
		let entry = self.devices.entry(mac.to_owned()).or_default();
		entry[0] += down;
		entry[1] += up;
	}
}
