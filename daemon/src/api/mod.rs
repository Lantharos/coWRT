mod devices;
mod live;
mod network;
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
		.route("/network/lan", get(network::lan))
		.route("/devices", get(devices::list))
		.route("/devices/{mac}", axum::routing::patch(devices::update))
		.route("/devices/{mac}/disconnect", post(devices::disconnect))
		.route("/wifi", get(wifi::overview))
		.route("/wifi/networks", post(wifi::create))
		.route("/wifi/networks/{id}", put(wifi::update).delete(wifi::delete))
		.route("/wifi/radios/{name}", put(wifi::update_radio))
		.route("/changes/{id}/confirm", post(confirm))
}

async fn confirm(State(state): State<AppState>, Path(id): Path<u64>) -> Result<(), ApiError> {
	state.applier.confirm(id).await
}
