mod auth;
mod error;
mod session;

use std::path::Path;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use tower_http::services::{ServeDir, ServeFile};

pub use error::{ApiError, ApiResult};
pub use session::Sessions;

use crate::api;
use crate::state::AppState;

pub fn router(state: AppState, www: &Path) -> Router {
	let api = api::routes()
		.route_layer(middleware::from_fn_with_state(state.clone(), auth::require))
		.merge(auth::routes())
		.merge(api::public_routes());

	let assets = ServeDir::new(www)
		.precompressed_br()
		.precompressed_gzip()
		.fallback(ServeFile::new(www.join("index.html")).precompressed_br().precompressed_gzip());

	Router::new()
		.nest("/api", api)
		.fallback_service(assets)
		.layer(middleware::from_fn(cache_policy))
		.with_state(state)
}

async fn cache_policy(request: Request, next: Next) -> Response {
	let immutable = request.uri().path().starts_with("/_app/immutable/");
	let mut response = next.run(request).await;
	let policy = if immutable {
		"public, max-age=31536000, immutable"
	} else {
		"no-cache"
	};
	response
		.headers_mut()
		.insert(header::CACHE_CONTROL, HeaderValue::from_static(policy));
	response
}
