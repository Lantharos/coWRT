use serde::Deserialize;
use serde_json::{Value, json};

use super::Security;
use crate::http::ApiError;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::Ubus;

const CONFIG: &str = "wireless";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInput {
	pub ssid: String,
	pub security: Security,
	pub password: Option<String>,
	pub hidden: bool,
	pub isolate: bool,
	pub network: String,
	pub enabled: bool,
	pub bands: Vec<String>,
	pub expires: Option<u64>,
}

impl NetworkInput {
	fn validate(&self) -> Result<(), ApiError> {
		if self.ssid.is_empty() || self.ssid.len() > 32 {
			return Err(ApiError::bad_request("Network names need 1 to 32 characters"));
		}
		if self.security.needs_password() {
			let length = self.password.as_deref().map_or(0, str::len);
			if !(8..=63).contains(&length) {
				return Err(ApiError::bad_request("Passwords need 8 to 63 characters"));
			}
		}
		if self.bands.is_empty() {
			return Err(ApiError::bad_request("Pick at least one band"));
		}
		Ok(())
	}

	fn values(&self, radio: &str) -> Value {
		let flag = |on: bool| if on { "1" } else { "0" };
		let mut values = json!({
			"device": radio,
			"mode": "ap",
			"ssid": self.ssid,
			"encryption": self.security.uci(),
			"key": if self.security.needs_password() { self.password.clone() } else { None },
			"hidden": flag(self.hidden),
			"isolate": flag(self.isolate),
			"network": self.network,
			"disabled": flag(!self.enabled),
		});
		if let Some(at) = self.expires {
			values["cowrt_expires"] = json!(at.to_string());
		}
		values
	}
}

async fn members(ubus: &Ubus, id: &str) -> Result<(Vec<Section>, Vec<Section>), ApiError> {
	let sections = uci::sections(ubus, CONFIG).await?;
	let ssid = sections
		.iter()
		.find(|s| s.name == id && s.kind == "wifi-iface")
		.and_then(|s| s.get("ssid").map(str::to_owned))
		.ok_or_else(|| ApiError::not_found("That network no longer exists"))?;
	let radios = sections.iter().filter(|s| s.kind == "wifi-device").cloned().collect();
	let members = sections
		.into_iter()
		.filter(|s| s.kind == "wifi-iface" && s.get("mode").unwrap_or("ap") == "ap" && s.get("ssid") == Some(&ssid))
		.collect();
	Ok((radios, members))
}

async fn write_members(ubus: &Ubus, input: &NetworkInput, radios: &[Section], members: &[Section]) -> Result<(), ApiError> {
	for radio in radios {
		let wanted = radio.get("band").is_some_and(|band| input.bands.iter().any(|b| b == band));
		let existing = members.iter().find(|m| m.get("device") == Some(&radio.name));
		match (wanted, existing) {
			(true, Some(member)) => {
				uci::set(ubus, CONFIG, &member.name, input.values(&radio.name)).await?;
				if !input.security.needs_password() {
					uci::unset(ubus, CONFIG, &member.name, &["key"]).await?;
				}
				if input.expires.is_none() {
					uci::unset(ubus, CONFIG, &member.name, &["cowrt_expires"]).await?;
				}
			}
			(true, None) => {
				uci::add(ubus, CONFIG, "wifi-iface", None, input.values(&radio.name)).await?;
			}
			(false, Some(member)) => uci::delete(ubus, CONFIG, &member.name).await?,
			(false, None) => {}
		}
	}
	super::roaming::sync(ubus, super::roaming::load(ubus).await?).await
}

pub async fn update_network(ubus: &Ubus, id: &str, input: &NetworkInput) -> Result<(), ApiError> {
	input.validate()?;
	let (radios, members) = members(ubus, id).await?;
	write_members(ubus, input, &radios, &members).await
}

pub async fn create_network(ubus: &Ubus, input: &NetworkInput) -> Result<(), ApiError> {
	input.validate()?;
	let sections = uci::sections(ubus, CONFIG).await?;
	if sections
		.iter()
		.any(|s| s.kind == "wifi-iface" && s.get("ssid") == Some(&input.ssid))
	{
		return Err(ApiError::conflict("A network with that name already exists"));
	}
	let radios: Vec<Section> = sections.into_iter().filter(|s| s.kind == "wifi-device").collect();
	write_members(ubus, input, &radios, &[]).await
}

pub async fn delete_network(ubus: &Ubus, id: &str) -> Result<(), ApiError> {
	let (_, members) = members(ubus, id).await?;
	for member in members {
		uci::delete(ubus, CONFIG, &member.name).await?;
	}
	Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadioInput {
	pub channel: String,
	pub htmode: Option<String>,
	pub txpower: Option<u32>,
	pub country: Option<String>,
	pub enabled: bool,
}

pub async fn update_radio(ubus: &Ubus, name: &str, input: &RadioInput) -> Result<(), ApiError> {
	if input.channel != "auto" && input.channel.parse::<u32>().is_err() {
		return Err(ApiError::bad_request("Channel must be a number or auto"));
	}
	if let Some(country) = &input.country
		&& (country.len() != 2 || !country.chars().all(|c| c.is_ascii_uppercase()))
	{
		return Err(ApiError::bad_request("Country must be a two-letter code"));
	}
	let mut values = json!({ "channel": input.channel, "disabled": if input.enabled { "0" } else { "1" } });
	let mut cleared = Vec::new();
	for (option, value) in [
		("htmode", input.htmode.clone()),
		("txpower", input.txpower.map(|p| p.to_string())),
		("country", input.country.clone()),
	] {
		match value {
			Some(value) => values[option] = json!(value),
			None => cleared.push(option),
		}
	}
	uci::set(ubus, CONFIG, name, values).await?;
	uci::unset(ubus, CONFIG, name, &cleared).await?;
	Ok(())
}
