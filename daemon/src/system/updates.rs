use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};

use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::state::AppState;
use crate::system::{release, upgrade};

const FIRST_CHECK: Duration = Duration::from_secs(120);
const TICK: Duration = Duration::from_secs(1800);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);
const NIGHT: std::ops::Range<i64> = 3..5;
const SECTION: &str = "updates";

#[derive(Clone, Default, Serialize)]
pub struct Available {
	pub cowrt: Option<String>,
	pub openwrt: Option<String>,
}

#[derive(Default)]
pub struct Updates {
	available: Mutex<Available>,
}

impl Updates {
	pub fn available(&self) -> Available {
		self.available.lock().expect("updates lock poisoned").clone()
	}

	fn set(&self, available: Available) {
		*self.available.lock().expect("updates lock poisoned") = available;
	}

	pub fn forget_cowrt(&self) {
		self.available.lock().expect("updates lock poisoned").cowrt = None;
	}
}

pub async fn automatic(ubus: &Ubus) -> Result<bool, ubus::Error> {
	Ok(uci::sections_of(ubus, "cowrt", SECTION)
		.await?
		.first()
		.is_some_and(|section| section.flag("auto")))
}

pub async fn set_automatic(ubus: &Ubus, enabled: bool) -> Result<(), ubus::Error> {
	uci::upsert(
		ubus,
		"cowrt",
		SECTION,
		SECTION,
		json!({ "auto": if enabled { "1" } else { "0" } }),
	)
	.await
}

pub async fn openwrt_version(ubus: &Ubus) -> Option<String> {
	let board = ubus.call("system", "board", json!({})).await.ok()?;
	Some(board.pointer("/release/version")?.as_str()?.to_owned())
}

async fn local_hour(ubus: &Ubus) -> Option<i64> {
	let info = ubus.call("system", "info", json!({})).await.ok()?;
	Some(info.get("localtime").and_then(Value::as_i64)? % 86_400 / 3600)
}

async fn check(state: &AppState) -> Option<release::Release> {
	let cowrt = release::latest().await.ok().filter(release::Release::is_newer);
	let openwrt = match (state.upgrade.latest().await, openwrt_version(&state.ubus).await) {
		(Ok(latest), Some(current)) if upgrade::newer(&latest, &current) => Some(latest),
		_ => None,
	};
	state.updates.set(Available {
		cowrt: cowrt.as_ref().map(|r| r.version().to_owned()),
		openwrt,
	});
	cowrt
}

async fn install_overnight(state: &AppState, release: &release::Release) {
	let night = local_hour(&state.ubus).await.is_some_and(|hour| NIGHT.contains(&hour));
	if !night || !automatic(&state.ubus).await.unwrap_or(false) {
		return;
	}
	match release::install(release).await {
		Ok(()) => eprintln!("cowrtd: installing coWRT {} overnight", release.version()),
		Err(err) => eprintln!("cowrtd: couldn't install coWRT {} overnight: {err}", release.version()),
	}
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		tokio::time::sleep(FIRST_CHECK).await;
		let mut pending = None;
		let mut checked: Option<Instant> = None;
		loop {
			if checked.is_none_or(|at| at.elapsed() >= CHECK_EVERY) {
				pending = check(&state).await;
				checked = Some(Instant::now());
			}
			if let Some(release) = &pending {
				install_overnight(&state, release).await;
			}
			tokio::time::sleep(TICK).await;
		}
	});
}
