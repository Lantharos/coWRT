use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::platform::ubus;

#[derive(Debug)]
pub struct ApiError {
	status: StatusCode,
	message: String,
}

impl std::fmt::Display for ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl ApiError {
	pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
		Self {
			status,
			message: message.into(),
		}
	}

	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, message)
	}

	pub fn not_found(message: impl Into<String>) -> Self {
		Self::new(StatusCode::NOT_FOUND, message)
	}

	pub fn conflict(message: impl Into<String>) -> Self {
		Self::new(StatusCode::CONFLICT, message)
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		(self.status, Json(json!({ "error": self.message }))).into_response()
	}
}

impl From<ubus::Error> for ApiError {
	fn from(err: ubus::Error) -> Self {
		Self::new(StatusCode::BAD_GATEWAY, err.to_string())
	}
}

impl From<std::io::Error> for ApiError {
	fn from(err: std::io::Error) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
	}
}

pub type ApiResult<T> = Result<Json<T>, ApiError>;
