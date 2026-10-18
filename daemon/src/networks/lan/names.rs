use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};

const PREFIX: &str = "cw_name_";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalName {
	id: String,
	name: String,
	ip: String,
}

#[derive(Deserialize)]
pub struct LocalNameInput {
	name: String,
	ip: Ipv4Addr,
}

pub async fn list(ubus: &Ubus) -> Result<Vec<LocalName>, ubus::Error> {
	Ok(uci::sections_of(ubus, "dhcp", "domain")
		.await?
		.into_iter()
		.filter(|s| s.name.starts_with(PREFIX))
		.filter_map(|s| {
			Some(LocalName {
				name: s.get("name")?.to_owned(),
				ip: s.get("ip")?.to_owned(),
				id: s.name,
			})
		})
		.collect())
}

fn valid_label(label: &str) -> bool {
	!label.is_empty()
		&& label.len() <= 63
		&& label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
		&& !label.starts_with('-')
		&& !label.ends_with('-')
}

pub async fn create(ubus: &Ubus, input: &LocalNameInput) -> Result<(), ApiError> {
	let name = input.name.trim().trim_end_matches('.').to_ascii_lowercase();
	if name.len() > 253 || !name.split('.').all(valid_label) {
		return Err(ApiError::bad_request(
			"Names use letters, numbers and dashes, like nas or printer.home",
		));
	}
	let existing = list(ubus).await?;
	if existing.iter().any(|entry| entry.name == name) {
		return Err(ApiError::conflict("That name is already taken"));
	}
	let id = format!("{PREFIX}{}", name.replace(['.', '-'], "_"));
	uci::add(
		ubus,
		"dhcp",
		"domain",
		Some(&id),
		json!({ "name": name, "ip": input.ip.to_string() }),
	)
	.await?;
	Ok(())
}

pub async fn delete(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	if !id.starts_with(PREFIX) {
		return Err(ApiError::not_found("That name doesn't exist"));
	}
	uci::delete(ubus, "dhcp", id).await?;
	Ok(())
}

pub async fn domain(ubus: &Ubus) -> Result<Option<String>, ubus::Error> {
	Ok(uci::sections_of(ubus, "dhcp", "dnsmasq")
		.await?
		.first()
		.and_then(|s| s.get("domain").map(str::to_owned)))
}
