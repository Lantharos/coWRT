use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::http::{ApiError, ApiResult, Sessions};
use crate::platform::config::Applied;
use crate::state::AppState;
use crate::system::account;
use crate::system::setup::{self, WifiSetup};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
	needed: bool,
	password_set: bool,
}

pub fn public_routes() -> Router<AppState> {
	Router::new()
		.route("/setup", get(status))
		.route("/setup/password", post(first_password))
}

async fn status(State(state): State<AppState>) -> ApiResult<Status> {
	Ok(Json(Status {
		needed: setup::needed(&state.ubus).await?,
		password_set: account::has_password().await,
	}))
}

#[derive(Deserialize)]
struct FirstPassword {
	password: String,
}

async fn first_password(State(state): State<AppState>, Json(input): Json<FirstPassword>) -> Result<Response, ApiError> {
	if account::has_password().await {
		return Err(ApiError::new(StatusCode::FORBIDDEN, "This router already has a password"));
	}
	account::set_password(&input.password).await?;
	let token = state.sessions.create().await;
	Ok(([(header::SET_COOKIE, Sessions::cookie(&token))], Json(())).into_response())
}

pub async fn wifi(State(state): State<AppState>, Json(input): Json<WifiSetup>) -> ApiResult<Applied> {
	let applied = state
		.applier
		.apply(&state.ubus, &["wireless"], None, async |ubus| {
			setup::configure_wifi(ubus, &input).await
		})
		.await?;
	Ok(Json(applied))
}

pub async fn done(State(state): State<AppState>) -> Result<(), ApiError> {
	setup::finish(&state).await
}
