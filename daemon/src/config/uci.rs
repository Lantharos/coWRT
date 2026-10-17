use serde_json::{Map, Value, json};

use crate::ubus::{self, Ubus};

#[derive(Debug, Clone)]
pub struct Section {
	pub name: String,
	pub kind: String,
	pub values: Map<String, Value>,
}

impl Section {
	pub fn get(&self, option: &str) -> Option<&str> {
		self.values.get(option)?.as_str()
	}

	pub fn flag(&self, option: &str) -> bool {
		matches!(self.get(option), Some("1" | "true" | "on" | "yes" | "enabled"))
	}

	pub fn list(&self, option: &str) -> Vec<String> {
		match self.values.get(option) {
			Some(Value::Array(items)) => items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect(),
			Some(Value::String(item)) => item.split_whitespace().map(str::to_owned).collect(),
			_ => Vec::new(),
		}
	}
}

pub async fn sections(ubus: &Ubus, config: &str) -> Result<Vec<Section>, ubus::Error> {
	let reply = match ubus.call("uci", "get", json!({ "config": config })).await {
		Err(ubus::Error::Status(ubus::STATUS_NOT_FOUND)) => return Ok(Vec::new()),
		reply => reply?,
	};
	let Some(Value::Object(values)) = reply.get("values") else {
		return Ok(Vec::new());
	};
	let mut sections: Vec<(i64, Section)> = values
		.iter()
		.filter_map(|(name, values)| {
			let values = values.as_object()?;
			let index = values.get(".index").and_then(Value::as_i64).unwrap_or_default();
			let kind = values.get(".type")?.as_str()?.to_owned();
			let values = values
				.iter()
				.filter(|(key, _)| !key.starts_with('.'))
				.map(|(k, v)| (k.clone(), v.clone()))
				.collect();
			Some((
				index,
				Section {
					name: name.clone(),
					kind,
					values,
				},
			))
		})
		.collect();
	sections.sort_by_key(|(index, _)| *index);
	Ok(sections.into_iter().map(|(_, section)| section).collect())
}

pub async fn sections_of(ubus: &Ubus, config: &str, kind: &str) -> Result<Vec<Section>, ubus::Error> {
	Ok(sections(ubus, config)
		.await?
		.into_iter()
		.filter(|section| section.kind == kind)
		.collect())
}

pub async fn set(ubus: &Ubus, config: &str, section: &str, values: Value) -> Result<(), ubus::Error> {
	ubus.call(
		"uci",
		"set",
		json!({ "config": config, "section": section, "values": values }),
	)
	.await?;
	Ok(())
}

pub async fn add(ubus: &Ubus, config: &str, kind: &str, name: Option<&str>, values: Value) -> Result<String, ubus::Error> {
	let mut args = json!({ "config": config, "type": kind, "values": values });
	if let Some(name) = name {
		args["name"] = json!(name);
	}
	let reply = ubus.call("uci", "add", args).await?;
	Ok(reply.get("section").and_then(Value::as_str).unwrap_or_default().to_owned())
}

pub async fn delete(ubus: &Ubus, config: &str, section: &str) -> Result<(), ubus::Error> {
	ubus.call("uci", "delete", json!({ "config": config, "section": section }))
		.await?;
	Ok(())
}

pub async fn unset(ubus: &Ubus, config: &str, section: &str, options: &[&str]) -> Result<(), ubus::Error> {
	match ubus
		.call(
			"uci",
			"delete",
			json!({ "config": config, "section": section, "options": options }),
		)
		.await
	{
		Ok(_) | Err(ubus::Error::Status(ubus::STATUS_NOT_FOUND)) => Ok(()),
		Err(err) => Err(err),
	}
}

pub async fn upsert(ubus: &Ubus, config: &str, kind: &str, name: &str, values: Value) -> Result<(), ubus::Error> {
	match set(ubus, config, name, values.clone()).await {
		Err(ubus::Error::Status(ubus::STATUS_NOT_FOUND)) => add(ubus, config, kind, Some(name), values).await.map(drop),
		result => result,
	}
}
