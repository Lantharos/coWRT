use std::process::Stdio;
use std::time::Duration;

use axum::extract::State;
use axum::{Json, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::uci;
use crate::http::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Settings {
	hostname: String,
	zonename: String,
	timezone: String,
}

fn valid_hostname(name: &str) -> bool {
	!name.is_empty() && name.len() <= 63 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !name.starts_with('-')
}

pub async fn update_settings(State(state): State<AppState>, Json(input): Json<Settings>) -> Result<(), ApiError> {
	if !valid_hostname(&input.hostname) {
		return Err(ApiError::bad_request("Names can use letters, numbers and dashes"));
	}
	if input.timezone.is_empty() || input.timezone.len() > 64 || input.timezone.contains(['\n', '\'']) {
		return Err(ApiError::bad_request("Pick a time zone from the list"));
	}
	let section = uci::sections_of(&state.ubus, "system", "system")
		.await?
		.into_iter()
		.next()
		.ok_or_else(|| ApiError::conflict("The system settings are missing"))?;
	state
		.applier
		.apply(&state.ubus, &["system"], None, async |ubus| {
			let values = json!({ "hostname": input.hostname, "zonename": input.zonename, "timezone": input.timezone });
			Ok(uci::set(ubus, "system", &section.name, values).await?)
		})
		.await?;
	let _ = Command::new("/etc/init.d/system").arg("reload").status().await;
	crate::policy::apply(&state.ubus).await?;
	Ok(())
}

#[derive(Deserialize)]
pub struct Password {
	current: String,
	next: String,
}

pub async fn password(State(state): State<AppState>, Json(input): Json<Password>) -> Result<(), ApiError> {
	if input.next.len() < 8 {
		return Err(ApiError::bad_request("Use at least 8 characters"));
	}
	if input.next.contains('\n') {
		return Err(ApiError::bad_request("Passwords can't contain line breaks"));
	}
	let check = state
		.ubus
		.call("session", "login", json!({ "username": "root", "password": input.current }))
		.await;
	let session = check.map_err(|_| ApiError::new(StatusCode::FORBIDDEN, "Your current password isn't right"))?;
	if let Some(id) = session.get("ubus_rpc_session").and_then(|id| id.as_str()) {
		let _ = state.ubus.call("session", "destroy", json!({ "ubus_rpc_session": id })).await;
	}
	let mut child = Command::new("passwd")
		.arg("root")
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.spawn()?;
	child
		.stdin
		.take()
		.expect("piped stdin")
		.write_all(format!("{0}\n{0}\n", input.next).as_bytes())
		.await?;
	let output = child.wait_with_output().await?;
	if !output.status.success() {
		return Err(ApiError::conflict(String::from_utf8_lossy(&output.stderr).trim().to_owned()));
	}
	Ok(())
}

pub async fn reboot(State(state): State<AppState>) -> Result<(), ApiError> {
	state.monitor.save().await;
	tokio::spawn(async move {
		tokio::time::sleep(Duration::from_secs(1)).await;
		let _ = state.ubus.call("system", "reboot", json!({})).await;
	});
	Ok(())
}
