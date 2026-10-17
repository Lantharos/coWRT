use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Applier;
use crate::http::Sessions;
use crate::monitor::Monitor;
use crate::ubus::Ubus;

pub const RUNTIME_DIR: &str = "/tmp/cowrt";

pub fn unix_now() -> u64 {
	SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[derive(Clone)]
pub struct AppState {
	pub ubus: Ubus,
	pub sessions: Arc<Sessions>,
	pub monitor: Arc<Monitor>,
	pub applier: Arc<Applier>,
}

impl AppState {
	pub async fn load() -> Self {
		let _ = tokio::fs::create_dir_all(RUNTIME_DIR).await;
		Self {
			ubus: Ubus::default(),
			sessions: Arc::new(Sessions::load().await),
			monitor: Arc::default(),
			applier: Arc::default(),
		}
	}
}
