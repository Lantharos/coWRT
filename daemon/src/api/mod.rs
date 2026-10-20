mod access;
pub mod devices;
mod internet;
mod live;
mod networks;
mod protection;
mod system;
mod wifi;

use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post, put};

use crate::http::ApiError;
use crate::state::AppState;

const UPLOAD_LIMIT: usize = 128 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
	Router::new()
		.route("/system", get(system::info))
		.route("/system/history", get(system::history))
		.route("/system/settings", put(system::update_settings))
		.route("/system/password", put(system::password))
		.route("/system/reboot", post(system::reboot))
		.route("/system/logs", get(system::logs))
		.route("/system/backup", get(system::backup))
		.route("/system/update", get(system::check_update).post(system::install_update))
		.route("/system/update/automatic", put(system::update_automatic))
		.route("/system/openwrt", get(system::check_openwrt).post(system::upgrade_openwrt))
		.route(
			"/system/restore",
			post(system::restore).layer(axum::extract::DefaultBodyLimit::max(UPLOAD_LIMIT)),
		)
		.route(
			"/system/firmware",
			post(system::firmware).layer(axum::extract::DefaultBodyLimit::max(UPLOAD_LIMIT)),
		)
		.route("/live", get(live::socket))
		.route("/setup/wifi", put(system::setup::wifi))
		.route("/setup/done", post(system::setup::done))
		.route("/internet", get(internet::overview))
		.route("/internet/connection", put(internet::update_connection))
		.route("/internet/qos", put(internet::update_qos))
		.route("/internet/ipv6", put(internet::update_ipv6))
		.route("/internet/uplinks", get(internet::uplinks::status))
		.route("/internet/uplinks/phone", post(internet::uplinks::add_phone))
		.route("/internet/uplinks/{id}", axum::routing::delete(internet::uplinks::remove))
		.route("/internet/failover", put(internet::uplinks::update_failover))
		.route("/internet/acceleration", put(internet::update_acceleration))
		.route("/internet/line", get(internet::line))
		.route(
			"/internet/speedtest",
			get(internet::speedtest).post(internet::start_speedtest),
		)
		.route("/sharing", get(access::sharing::overview))
		.route("/forwards", post(access::sharing::create))
		.route("/forwards/{id}", put(access::sharing::update).delete(access::sharing::delete))
		.route("/forwards/check", post(access::sharing::check))
		.route("/upnp", put(access::sharing::set_upnp))
		.route("/ddns", put(access::sharing::set_ddns))
		.route("/protection", get(protection::overview).put(protection::update))
		.route("/protection/dns", put(protection::update_dns))
		.route(
			"/protection/activity",
			get(protection::activity).put(protection::set_activity),
		)
		.route("/vpn", get(access::vpn::overview))
		.route("/vpn/server", put(access::vpn::update_server))
		.route("/vpn/server/peers", post(access::vpn::add_peer))
		.route("/vpn/server/peers/{id}", axum::routing::delete(access::vpn::remove_peer))
		.route("/vpn/server/peers/{id}/config", get(access::vpn::peer_config))
		.route("/vpn/client", put(access::vpn::update_client))
		.route("/networks", get(networks::overview).post(networks::create))
		.route("/networks/{id}", put(networks::update).delete(networks::delete))
		.route("/ports/{name}", put(networks::update_port))
		.route("/people", get(devices::people::list).post(devices::people::create))
		.route("/people/{id}", put(devices::people::update).delete(devices::people::delete))
		.route("/people/{id}/activity", get(devices::activity::person))
		.route("/devices", get(devices::list))
		.route("/devices/{mac}", axum::routing::patch(devices::update))
		.route("/devices/{mac}/disconnect", post(devices::disconnect))
		.route("/devices/{mac}/wake", post(networks::lan::wake))
		.route("/devices/{mac}/activity", get(devices::activity::device))
		.route("/lan", get(networks::lan::overview))
		.route("/lan/names", post(networks::lan::create_name))
		.route("/lan/names/{id}", axum::routing::delete(networks::lan::delete_name))
		.route("/lan/routes", post(networks::lan::create_route))
		.route("/lan/routes/{id}", axum::routing::delete(networks::lan::delete_route))
		.route("/wifi", get(wifi::overview))
		.route("/wifi/networks", post(wifi::create))
		.route("/wifi/networks/{id}", put(wifi::update).delete(wifi::delete))
		.route("/wifi/radios/{name}", put(wifi::update_radio))
		.route("/wifi/radios/{name}/scan", post(wifi::scan))
		.route("/wifi/roaming", put(wifi::update_roaming))
}

pub fn public_routes() -> Router<AppState> {
	Router::new()
		.route("/changes/{token}/confirm", post(confirm))
		.merge(system::setup::public_routes())
}

async fn confirm(State(state): State<AppState>, Path(token): Path<String>) -> Result<(), ApiError> {
	state.applier.confirm(&token).await
}
