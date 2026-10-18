use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::ApiError;
use crate::networks::{Cidr, Layout};
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};

const PREFIX: &str = "cw_route_";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
	id: String,
	target: String,
	gateway: String,
	network: String,
}

#[derive(Deserialize)]
pub struct RouteInput {
	target: String,
	gateway: Ipv4Addr,
}

pub async fn list(ubus: &Ubus) -> Result<Vec<Route>, ubus::Error> {
	Ok(uci::sections_of(ubus, "network", "route")
		.await?
		.into_iter()
		.filter(|s| s.name.starts_with(PREFIX))
		.filter_map(|s| {
			Some(Route {
				target: s.get("target")?.to_owned(),
				gateway: s.get("gateway")?.to_owned(),
				network: s.get("interface")?.to_owned(),
				id: s.name,
			})
		})
		.collect())
}

pub async fn create(ubus: &Ubus, input: &RouteInput) -> Result<(), ApiError> {
	let target: Cidr = input
		.target
		.trim()
		.parse()
		.map_err(|_| ApiError::bad_request("Destinations look like 192.168.50.0/24"))?;
	let network = Layout::load(ubus)
		.await?
		.subnets()
		.into_iter()
		.find(|(_, subnet)| subnet.contains(input.gateway))
		.map(|(id, _)| id)
		.ok_or_else(|| ApiError::bad_request("The gateway has to be on one of your networks"))?;
	let id = format!("{PREFIX}{}", &crate::system::secret::token()[..8]);
	let values = json!({
		"interface": network,
		"target": target.canonical().to_string(),
		"gateway": input.gateway.to_string(),
	});
	uci::add(ubus, "network", "route", Some(&id), values).await?;
	Ok(())
}

pub async fn delete(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	if !id.starts_with(PREFIX) {
		return Err(ApiError::not_found("That route doesn't exist"));
	}
	uci::delete(ubus, "network", id).await?;
	Ok(())
}
