use serde::Deserialize;
use serde_json::json;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::state::AppState;
use crate::wifi::{self, NetworkInput, Security};

const SECTION: &str = "setup";
const STOCK_SSID: &str = "OpenWrt";
const PRIMARY: &str = "lan";

#[derive(Deserialize)]
pub struct WifiSetup {
	ssid: String,
	password: String,
	country: String,
}

pub async fn needed(ubus: &Ubus) -> Result<bool, ubus::Error> {
	let sections = uci::sections_of(ubus, "cowrt", SECTION).await?;
	Ok(!sections.first().is_some_and(|s| s.flag("done")))
}

async fn mark_done(ubus: &Ubus) -> Result<(), ApiError> {
	uci::upsert(ubus, "cowrt", SECTION, SECTION, json!({ "done": "1" })).await?;
	Ok(())
}

pub async fn finish(state: &AppState) -> Result<(), ApiError> {
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| mark_done(ubus).await)
		.await?;
	Ok(())
}

pub async fn skip_if_configured(state: &AppState) -> Result<(), ApiError> {
	if !needed(&state.ubus).await? || !crate::system::account::has_password().await {
		return Ok(());
	}
	let stock = uci::sections_of(&state.ubus, "wireless", "wifi-iface")
		.await?
		.iter()
		.any(|iface| iface.get("ssid") == Some(STOCK_SSID));
	if stock {
		return Ok(());
	}
	finish(state).await
}

pub async fn configure_wifi(ubus: &Ubus, input: &WifiSetup) -> Result<(), ApiError> {
	if input.country.len() != 2 || !input.country.chars().all(|c| c.is_ascii_uppercase()) {
		return Err(ApiError::bad_request("Pick the country the router is in"));
	}
	let radios = uci::sections_of(ubus, "wireless", "wifi-device").await?;
	for radio in &radios {
		uci::set(
			ubus,
			"wireless",
			&radio.name,
			json!({ "disabled": "0", "country": input.country }),
		)
		.await?;
	}
	let mut bands: Vec<String> = radios.iter().filter_map(|r| r.get("band").map(str::to_owned)).collect();
	bands.sort();
	bands.dedup();
	let network = NetworkInput {
		ssid: input.ssid.trim().to_owned(),
		security: Security::Wpa2Wpa3,
		password: Some(input.password.clone()),
		hidden: false,
		isolate: false,
		network: PRIMARY.to_owned(),
		enabled: true,
		bands,
		expires: None,
	};
	let overview = wifi::overview(ubus).await?;
	match overview.networks.iter().find(|n| n.network == PRIMARY) {
		Some(existing) => wifi::update_network(ubus, &existing.id, &network).await,
		None => wifi::create_network(ubus, &network).await,
	}
}
