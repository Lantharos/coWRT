use tokio::fs;

#[derive(Clone, Copy, Default)]
pub struct CpuTimes {
	busy: u64,
	total: u64,
}

impl CpuTimes {
	pub async fn read() -> Option<Self> {
		let stat = fs::read_to_string("/proc/stat").await.ok()?;
		let fields: Vec<u64> = stat
			.lines()
			.next()?
			.split_whitespace()
			.skip(1)
			.filter_map(|v| v.parse().ok())
			.collect();
		let total: u64 = fields.iter().take(8).sum();
		let idle = fields.get(3)? + fields.get(4).unwrap_or(&0);
		Some(Self {
			busy: total - idle,
			total,
		})
	}

	pub fn usage_since(self, earlier: Self) -> f32 {
		let total = self.total.saturating_sub(earlier.total);
		if total == 0 {
			return 0.0;
		}
		self.busy.saturating_sub(earlier.busy) as f32 / total as f32 * 100.0
	}
}

#[derive(Clone, Copy, Default)]
pub struct Memory {
	pub total: u64,
	pub available: u64,
}

impl Memory {
	pub async fn read() -> Option<Self> {
		let info = fs::read_to_string("/proc/meminfo").await.ok()?;
		let field = |name: &str| -> Option<u64> {
			let line = info.lines().find(|line| line.starts_with(name))?;
			Some(line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024)
		};
		Some(Self {
			total: field("MemTotal:")?,
			available: field("MemAvailable:")?,
		})
	}
}

#[derive(Clone, Copy, Default)]
pub struct Counters {
	pub rx: u64,
	pub tx: u64,
}

impl Counters {
	pub async fn read(device: &str) -> Option<Self> {
		let read = async |name: &str| -> Option<u64> {
			fs::read_to_string(format!("/sys/class/net/{device}/statistics/{name}"))
				.await
				.ok()?
				.trim()
				.parse()
				.ok()
		};
		Some(Self {
			rx: read("rx_bytes").await?,
			tx: read("tx_bytes").await?,
		})
	}

	pub fn since(self, earlier: Self) -> Self {
		Self {
			rx: self.rx.saturating_sub(earlier.rx),
			tx: self.tx.saturating_sub(earlier.tx),
		}
	}
}
