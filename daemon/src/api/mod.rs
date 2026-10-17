pub mod devices;
mod internet;
mod live;
mod networks;
mod sharing;
mod system;
mod wifi;

use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post, put};

use crate::http::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
	Router::new()
		.route("/system", get(system::info))
		.route("/system/history", get(system::history))
		.route("/live", get(live::socket))
		.route("/internet", get(internet::overview))
		.route("/internet/connection", put(internet::update_connection))
		.route("/internet/qos", put(internet::update_qos))
		.route("/internet/acceleration", put(internet::update_acceleration))
		.route("/sharing", get(sharing::overview))
		.route("/forwards", post(sharing::create))
		.route("/forwards/{id}", put(sharing::update).delete(sharing::delete))
		.route("/forwards/check", post(sharing::check))
		.route("/upnp", put(sharing::set_upnp))
		.route("/ddns", put(sharing::set_ddns))
		.route("/networks", get(networks::overview).post(networks::create))
		.route("/networks/{id}", put(networks::update).delete(networks::delete))
		.route("/ports/{name}", put(networks::update_port))
		.route("/devices", get(devices::list))
		.route("/devices/{mac}", axum::routing::patch(devices::update))
		.route("/devices/{mac}/disconnect", post(devices::disconnect))
		.route("/wifi", get(wifi::overview))
		.route("/wifi/networks", post(wifi::create))
		.route("/wifi/networks/{id}", put(wifi::update).delete(wifi::delete))
		.route("/wifi/radios/{name}", put(wifi::update_radio))
}

pub fn public_routes() -> Router<AppState> {
	Router::new().route("/changes/{token}/confirm", post(confirm))
}

async fn confirm(State(state): State<AppState>, Path(token): Path<String>) -> Result<(), ApiError> {
	state.applier.confirm(&token).await
}
