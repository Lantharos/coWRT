use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::error::ApiError;
use super::session::{COOKIE, Sessions};
use crate::platform::ubus;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
	Router::new()
		.route("/auth/login", post(login))
		.route("/auth/logout", post(logout))
		.route("/auth/session", get(session))
}

fn token(headers: &HeaderMap) -> Option<&str> {
	headers
		.get_all(header::COOKIE)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(';'))
		.find_map(|pair| pair.trim().strip_prefix(COOKIE)?.strip_prefix('='))
}

pub async fn require(State(state): State<AppState>, request: Request, next: Next) -> Response {
	match token(request.headers()) {
		Some(token) if state.sessions.validate(token).await => next.run(request).await,
		_ => ApiError::new(StatusCode::UNAUTHORIZED, "Sign in to continue").into_response(),
	}
}

#[derive(Deserialize)]
struct Login {
	password: String,
}

async fn login(State(state): State<AppState>, Json(body): Json<Login>) -> Result<Response, ApiError> {
	let attempt = state
		.ubus
		.call("session", "login", json!({ "username": "root", "password": body.password }))
		.await;
	match attempt {
		Ok(reply) => {
			if let Some(id) = reply.get("ubus_rpc_session").and_then(|id| id.as_str()) {
				state
					.ubus
					.call("session", "destroy", json!({ "ubus_rpc_session": id }))
					.await?;
			}
			let token = state.sessions.create().await;
			Ok((
				[(header::SET_COOKIE, Sessions::cookie(&token))],
				Json(SessionInfo { authenticated: true }),
			)
				.into_response())
		}
		Err(ubus::Error::Status(_)) => {
			state.sessions.penalize().await;
			Err(ApiError::new(StatusCode::UNAUTHORIZED, "That password didn't work"))
		}
		Err(err) => Err(err.into()),
	}
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
	if let Some(token) = token(&headers) {
		state.sessions.revoke(token).await;
	}
	(
		[(header::SET_COOKIE, Sessions::expired_cookie())],
		Json(SessionInfo { authenticated: false }),
	)
		.into_response()
}

#[derive(Serialize)]
struct SessionInfo {
	authenticated: bool,
}

async fn session(State(state): State<AppState>, headers: HeaderMap) -> Json<SessionInfo> {
	let authenticated = match token(&headers) {
		Some(token) => state.sessions.validate(token).await,
		None => false,
	};
	Json(SessionInfo { authenticated })
}
