mod parse;
mod store;

use std::collections::HashMap;
use std::ffi::CString;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use store::Store;
pub use store::{Bucket, Counts, Site};

use crate::monitor::wan::utc_offset;
use crate::platform::netlink::neighbors;
use crate::protection;
use crate::state::{AppState, RUNTIME_DIR, unix_now};

const EVERY: Duration = Duration::from_secs(10);
const SETTLE: Duration = Duration::from_millis(200);
const RESOLVER_USER: &str = "nobody";
const SAVE_EVERY: u64 = 6;

fn saved() -> std::path::PathBuf {
	Path::new(RUNTIME_DIR).join("activity.json")
}

#[derive(Default)]
pub struct Activity {
	store: Mutex<Store>,
}

impl Activity {
	async fn restore(&self) {
		let raw = tokio::fs::read(saved()).await.unwrap_or_default();
		if let Ok(store) = serde_json::from_slice(&raw) {
			*self.store.lock().expect("activity lock poisoned") = store;
		}
	}

	pub async fn save(&self) {
		let json = serde_json::to_vec(&*self.store.lock().expect("activity lock poisoned"));
		if let Ok(json) = json {
			let _ = tokio::fs::write(saved(), json).await;
		}
	}

	pub fn sites(&self, macs: &[String], limit: usize) -> (Vec<Site>, Counts) {
		self.store.lock().expect("activity lock poisoned").sites(macs, limit)
	}

	pub fn top_blocked(&self, limit: usize) -> Vec<Site> {
		self.store.lock().expect("activity lock poisoned").top_blocked(limit)
	}

	pub fn buckets(&self) -> Vec<Bucket> {
		self.store.lock().expect("activity lock poisoned").buckets()
	}
}

fn hand_to_resolver(path: &Path) {
	let (Ok(user), Ok(file)) = (CString::new(RESOLVER_USER), CString::new(path.as_os_str().as_encoded_bytes())) else {
		return;
	};
	unsafe {
		let entry = libc::getpwnam(user.as_ptr());
		if !entry.is_null() {
			libc::chown(file.as_ptr(), (*entry).pw_uid, (*entry).pw_gid);
		}
	}
}

async fn collect() -> String {
	let Ok(mut entries) = tokio::fs::read_dir(protection::dns_dir()).await else {
		return String::new();
	};
	let mut taken = Vec::new();
	while let Ok(Some(entry)) = entries.next_entry().await {
		let path = entry.path();
		if path.extension().is_none_or(|ext| ext != "log") {
			continue;
		}
		let reading = path.with_extension("reading");
		if tokio::fs::rename(&path, &reading).await.is_ok() {
			if tokio::fs::File::create(&path).await.is_ok() {
				hand_to_resolver(&path);
			}
			taken.push(reading);
		}
	}
	if taken.is_empty() {
		return String::new();
	}
	protection::reopen_logs().await;
	tokio::time::sleep(SETTLE).await;
	let mut text = String::new();
	for path in taken {
		text.push_str(&tokio::fs::read_to_string(&path).await.unwrap_or_default());
		let _ = tokio::fs::remove_file(&path).await;
	}
	text
}

async fn owners() -> HashMap<IpAddr, String> {
	let Ok(Ok(neighbors)) = tokio::task::spawn_blocking(neighbors::list).await else {
		return HashMap::new();
	};
	neighbors.into_iter().map(|n| (n.ip, n.mac)).collect()
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		let mut ticker = tokio::time::interval(EVERY);
		ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		let mut offset = 0;
		let mut round = 0u64;
		state.activity.restore().await;
		loop {
			ticker.tick().await;
			round += 1;
			if !protection::activity_enabled(&state.ubus).await.unwrap_or(false) {
				continue;
			}
			offset = utc_offset(&state.ubus).await.unwrap_or(offset);
			let text = collect().await;
			let owners = owners().await;
			let now = unix_now();
			{
				let mut store = state.activity.store.lock().expect("activity lock poisoned");
				store.roll_over((now as i64 + offset) as u64 / 86_400);
				store.record(&text, &owners, now);
			}
			if round.is_multiple_of(SAVE_EVERY) {
				state.activity.save().await;
			}
		}
	});
}
