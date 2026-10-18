use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LogQuery {
	#[serde(default = "default_lines")]
	lines: u32,
}

fn default_lines() -> u32 {
	300
}

#[derive(Serialize)]
pub struct Entry {
	time: u64,
	level: &'static str,
	message: String,
}

fn level(priority: u64) -> &'static str {
	match priority & 0x7 {
		0..=3 => "error",
		4 => "warning",
		5 | 6 => "info",
		_ => "debug",
	}
}

pub async fn logs(State(state): State<AppState>, Query(query): Query<LogQuery>) -> ApiResult<Vec<Entry>> {
	let reply = state
		.ubus
		.call(
			"log",
			"read",
			json!({ "lines": query.lines.min(2000), "stream": false, "oneshot": true }),
		)
		.await?;
	let entries = reply
		.get("log")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|entry| {
			Some(Entry {
				time: entry.get("time")?.as_u64()? / 1000,
				level: level(entry.get("priority")?.as_u64()?),
				message: entry.get("msg")?.as_str()?.to_owned(),
			})
		})
		.collect();
	Ok(Json(entries))
}
