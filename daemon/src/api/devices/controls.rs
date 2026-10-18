use serde_json::json;

use super::DeviceUpdate;
use crate::devices::policy;
use crate::devices::section_id;
use crate::http::ApiError;
use crate::platform::config::uci;
use crate::state::AppState;

const MAX_LIMIT_KBIT: u32 = 10_000_000;

fn validate(body: &DeviceUpdate) -> Result<(), ApiError> {
	for schedule in body.schedules.iter().flatten() {
		schedule.validate().map_err(ApiError::bad_request)?;
	}
	for limit in [body.limit_down, body.limit_up].into_iter().flatten().flatten() {
		if limit == 0 || limit > MAX_LIMIT_KBIT {
			return Err(ApiError::bad_request("Speed limits go from 1 Kbps to 10 Gbps"));
		}
	}
	Ok(())
}

pub async fn set(state: &AppState, mac: &str, body: &DeviceUpdate) -> Result<(), ApiError> {
	let touched = body.blocked.is_some()
		|| body.schedules.is_some()
		|| body.limit_down.is_some()
		|| body.limit_up.is_some()
		|| body.priority.is_some();
	if !touched {
		return Ok(());
	}
	validate(body)?;
	let section = format!("d{}", section_id(mac));
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			uci::upsert(ubus, "cowrt", "device", &section, json!({ "mac": mac })).await?;
			if let Some(blocked) = body.blocked {
				uci::set(ubus, "cowrt", &section, json!({ "blocked": if blocked { "1" } else { "0" } })).await?;
			}
			if let Some(schedules) = &body.schedules {
				uci::unset(ubus, "cowrt", &section, &["schedule"]).await?;
				if !schedules.is_empty() {
					let entries: Vec<String> = schedules.iter().map(|s| s.to_uci()).collect();
					uci::set(ubus, "cowrt", &section, json!({ "schedule": entries })).await?;
				}
			}
			for (option, value) in [("limit_down", body.limit_down), ("limit_up", body.limit_up)] {
				match value {
					Some(Some(kbit)) => uci::set(ubus, "cowrt", &section, json!({ option: kbit.to_string() })).await?,
					Some(None) => uci::unset(ubus, "cowrt", &section, &[option]).await?,
					None => {}
				}
			}
			if let Some(priority) = body.priority {
				match priority.uci() {
					Some(value) => uci::set(ubus, "cowrt", &section, json!({ "priority": value })).await?,
					None => uci::unset(ubus, "cowrt", &section, &["priority"]).await?,
				}
			}
			Ok(())
		})
		.await?;
	policy::apply(&state.ubus)
		.await
		.map_err(|err| ApiError::conflict(format!("The router rejected the new rules: {err}")))?;
	if body.blocked == Some(true) {
		policy::cut_off(mac).await;
	}
	Ok(())
}
