use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::devices::policy::Schedule;
use crate::http::ApiError;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

const COLORS: [&str; 6] = ["green", "blue", "purple", "orange", "pink", "teal"];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
	pub id: String,
	pub name: String,
	pub color: String,
	pub blocked: bool,
	pub schedules: Vec<Schedule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonInput {
	name: String,
	color: String,
	pub blocked: bool,
	schedules: Vec<Schedule>,
}

impl Person {
	fn from_section(section: &Section) -> Self {
		Self {
			id: section.name.clone(),
			name: section.get("name").unwrap_or(&section.name).to_owned(),
			color: section.get("color").unwrap_or(COLORS[0]).to_owned(),
			blocked: section.flag("blocked"),
			schedules: section
				.list("schedule")
				.iter()
				.filter_map(|entry| Schedule::parse(entry))
				.collect(),
		}
	}
}

pub async fn list(ubus: &Ubus) -> Result<Vec<Person>, ubus::Error> {
	Ok(uci::sections_of(ubus, "cowrt", "person")
		.await?
		.iter()
		.map(Person::from_section)
		.collect())
}

fn validate(input: &PersonInput) -> Result<(), ApiError> {
	let name = input.name.trim();
	if name.is_empty() || name.chars().count() > 32 {
		return Err(ApiError::bad_request("Names are 1 to 32 characters"));
	}
	if !COLORS.contains(&input.color.as_str()) {
		return Err(ApiError::bad_request("Pick one of the colors"));
	}
	for schedule in &input.schedules {
		schedule.validate().map_err(ApiError::bad_request)?;
	}
	Ok(())
}

pub fn new_id() -> String {
	format!("p{}", &crate::system::secret::token()[..8])
}

pub async fn store(ubus: &Ubus, id: &str, input: &PersonInput) -> Result<(), ApiError> {
	validate(input)?;
	let values = json!({
		"name": input.name.trim(),
		"color": input.color,
		"blocked": if input.blocked { "1" } else { "0" },
	});
	uci::upsert(ubus, "cowrt", "person", id, values).await?;
	uci::unset(ubus, "cowrt", id, &["schedule"]).await?;
	if !input.schedules.is_empty() {
		let entries: Vec<String> = input.schedules.iter().map(Schedule::to_uci).collect();
		uci::set(ubus, "cowrt", id, json!({ "schedule": entries })).await?;
	}
	Ok(())
}

pub async fn remove(ubus: &Ubus, id: &str) -> Result<(), ubus::Error> {
	for device in uci::sections_of(ubus, "cowrt", "device").await? {
		if device.get("owner") == Some(id) {
			uci::unset(ubus, "cowrt", &device.name, &["owner"]).await?;
		}
	}
	uci::delete(ubus, "cowrt", id).await
}
