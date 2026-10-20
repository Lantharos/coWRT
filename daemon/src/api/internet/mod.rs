pub mod uplinks;

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::http::{ApiError, ApiResult};
use crate::internet::qos::{self, Settings};
use crate::internet::speedtest;
use crate::internet::{self, Acceleration, Connection, Ipv6Settings};
use crate::monitor::wan::Wan;
use crate::networks::ports;
use crate::platform::config::Applied;
use crate::state::AppState;

const ROLLBACK: Option<Duration> = Some(Duration::from_secs(60));

#[derive(Serialize)]
pub struct Overview {
	connection: Connection,
	ports: Vec<String>,
	qos: Settings,
	acceleration: Acceleration,
	ipv6: Ipv6Settings,
}

pub async fn overview(State(state): State<AppState>) -> ApiResult<Overview> {
	Ok(Json(Overview {
		connection: internet::connection(&state.ubus).await?,
		ports: ports::physical().await,
		qos: qos::load(&state.ubus).await?,
		acceleration: internet::acceleration(&state.ubus).await?,
		ipv6: internet::ipv6(&state.ubus).await?,
	}))
}

pub async fn update_connection(State(state): State<AppState>, Json(input): Json<Connection>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["network"], ROLLBACK, async |ubus| {
			internet::store(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn update_ipv6(State(state): State<AppState>, Json(input): Json<Ipv6Settings>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["network"], ROLLBACK, async |ubus| {
			internet::set_ipv6(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn update_qos(State(state): State<AppState>, Json(input): Json<Settings>) -> ApiResult<Applied> {
	let measured = input.mode == qos::Mode::Speedtest;
	if input.enabled && !measured && (input.download < 1000 || input.upload < 1000) {
		return Err(ApiError::bad_request("Enter your line speeds, at least 1 Mbps each way"));
	}
	let bypassed = input.enabled && internet::acceleration(&state.ubus).await? == Acceleration::Hardware;
	let applied = state
		.applier
		.apply(&state.ubus, &["cowrt", "firewall"], None, async |ubus| {
			qos::store(ubus, &input).await?;
			if bypassed {
				internet::set_acceleration(ubus, Acceleration::Software).await?;
			}
			Ok(())
		})
		.await?;
	let wan = Wan::read(&state.ubus, crate::monitor::wan::MAIN).await;
	qos::apply(&state.ubus, wan.device.as_deref())
		.await
		.map_err(|err| ApiError::conflict(format!("The smart queue couldn't start: {err}")))?;
	if input.enabled && measured {
		state.speedtest.start(state.clone());
	}
	Ok(Json(applied))
}

pub async fn line(State(state): State<AppState>) -> Json<crate::internet::line::View> {
	Json(state.line.view().await)
}

pub async fn speedtest(State(state): State<AppState>) -> Json<speedtest::Overview> {
	Json(state.speedtest.overview().await)
}

pub async fn start_speedtest(State(state): State<AppState>) -> Result<(), ApiError> {
	if state.speedtest.start(state.clone()) {
		Ok(())
	} else {
		Err(ApiError::conflict("A speed test is already running"))
	}
}

#[derive(Deserialize)]
pub struct AccelerationInput {
	mode: Acceleration,
}

pub async fn update_acceleration(State(state): State<AppState>, Json(input): Json<AccelerationInput>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["firewall"], None, async |ubus| {
			internet::set_acceleration(ubus, input.mode).await
		})
		.await?;
	Ok(Json(applied))
}
