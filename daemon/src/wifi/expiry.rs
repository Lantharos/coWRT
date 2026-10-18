use serde_json::json;

use crate::platform::config::uci;
use crate::state::{AppState, unix_now};

const OPTION: &str = "cowrt_expires";

pub async fn turn_off_expired(state: &AppState) {
	let Ok(ifaces) = uci::sections_of(&state.ubus, "wireless", "wifi-iface").await else {
		return;
	};
	let now = unix_now();
	let due: Vec<String> = ifaces
		.iter()
		.filter(|iface| {
			iface
				.get(OPTION)
				.and_then(|at| at.parse::<u64>().ok())
				.is_some_and(|at| at <= now)
		})
		.map(|iface| iface.name.clone())
		.collect();
	if due.is_empty() {
		return;
	}
	let _ = state
		.applier
		.apply(&state.ubus, &["wireless"], None, async |ubus| {
			for name in &due {
				uci::set(ubus, "wireless", name, json!({ "disabled": "1" })).await?;
				uci::unset(ubus, "wireless", name, &[OPTION]).await?;
			}
			Ok(())
		})
		.await;
}
