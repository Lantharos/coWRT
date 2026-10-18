use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Notify;

use crate::access::forwards::reach;
use crate::monitor::wan::Wan;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::state::{AppState, unix_now};
use crate::system::outbound::{Method, Request};

const SECTION: &str = "ddns";
const API: &str = "https://api.cloudflare.com/client/v4";
const INTERVAL: Duration = Duration::from_secs(300);

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
	pub ipv4: Option<String>,
	pub ipv6: Option<String>,
	pub updated: Option<u64>,
	pub error: Option<String>,
}

static STATUS: LazyLock<Mutex<Status>> = LazyLock::new(Mutex::default);
static WAKE: LazyLock<Notify> = LazyLock::new(Notify::new);

#[derive(Clone, Default)]
pub struct Settings {
	pub enabled: bool,
	pub hostname: String,
	pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	enabled: bool,
	hostname: String,
	has_token: bool,
	status: Status,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
	pub enabled: bool,
	pub hostname: String,
	pub token: Option<String>,
}

pub async fn load(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let sections = uci::sections_of(ubus, "cowrt", SECTION).await?;
	Ok(sections
		.first()
		.map(|s| Settings {
			enabled: s.flag("enabled"),
			hostname: s.get("hostname").unwrap_or_default().to_owned(),
			token: s.get("token").unwrap_or_default().to_owned(),
		})
		.unwrap_or_default())
}

pub async fn view(ubus: &Ubus) -> Result<View, ubus::Error> {
	let settings = load(ubus).await?;
	Ok(View {
		enabled: settings.enabled,
		hostname: settings.hostname,
		has_token: !settings.token.is_empty(),
		status: STATUS.lock().expect("ddns status").clone(),
	})
}

pub async fn store(ubus: &Ubus, input: &Input) -> Result<(), ubus::Error> {
	let mut values =
		json!({ "enabled": if input.enabled { "1" } else { "0" }, "hostname": input.hostname.trim().to_ascii_lowercase() });
	if let Some(token) = input.token.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
		values["token"] = json!(token);
	}
	uci::upsert(ubus, "cowrt", SECTION, SECTION, values).await
}

pub fn refresh() {
	WAKE.notify_one();
}

async fn cloudflare(token: &str, method: Method, path: &str, body: Option<Value>) -> Result<Value, String> {
	let reply: Value = Request::new(method, format!("{API}{path}"))
		.header("Authorization", format!("Bearer {token}"))
		.json(body)
		.await
		.map_err(|err| err.to_string())?;
	if reply.get("success").and_then(Value::as_bool) == Some(true) {
		return Ok(reply["result"].clone());
	}
	let message = reply
		.pointer("/errors/0/message")
		.and_then(Value::as_str)
		.unwrap_or("Cloudflare refused the request");
	Err(message.to_owned())
}

async fn zone_of(token: &str, hostname: &str) -> Result<String, String> {
	let labels: Vec<&str> = hostname.split('.').collect();
	for start in 0..labels.len().saturating_sub(1) {
		let candidate = labels[start..].join(".");
		let zones = cloudflare(token, Method::Get, &format!("/zones?name={candidate}"), None).await?;
		if let Some(id) = zones.pointer("/0/id").and_then(Value::as_str) {
			return Ok(id.to_owned());
		}
	}
	Err(format!("No Cloudflare zone for {hostname} is reachable with this token"))
}

async fn upsert_record(token: &str, zone: &str, hostname: &str, kind: &str, content: &str) -> Result<(), String> {
	let records = cloudflare(
		token,
		Method::Get,
		&format!("/zones/{zone}/dns_records?type={kind}&name={hostname}"),
		None,
	)
	.await?;
	let body = json!({ "type": kind, "name": hostname, "content": content, "ttl": 60, "proxied": false });
	match records.pointer("/0") {
		Some(record) if record.get("content").and_then(Value::as_str) == Some(content) => Ok(()),
		Some(record) => {
			let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
			cloudflare(token, Method::Patch, &format!("/zones/{zone}/dns_records/{id}"), Some(body))
				.await
				.map(drop)
		}
		None => cloudflare(token, Method::Post, &format!("/zones/{zone}/dns_records"), Some(body))
			.await
			.map(drop),
	}
}

async fn update(ubus: &Ubus, settings: &Settings, forced: bool) -> Result<Status, String> {
	let ipv4 = reach::public_ipv4()
		.await
		.map_err(|err| format!("Couldn't find your public address: {err}"))?;
	let ipv6 = Wan::read(ubus)
		.await
		.ipv6
		.map(|cidr| cidr.split('/').next().unwrap_or_default().to_owned());
	let previous = STATUS.lock().expect("ddns status").clone();
	if !forced && previous.error.is_none() && previous.ipv4.as_deref() == Some(&ipv4) && previous.ipv6 == ipv6 {
		return Ok(previous);
	}
	let zone = zone_of(&settings.token, &settings.hostname).await?;
	upsert_record(&settings.token, &zone, &settings.hostname, "A", &ipv4).await?;
	if let Some(ipv6) = &ipv6 {
		upsert_record(&settings.token, &zone, &settings.hostname, "AAAA", ipv6).await?;
	}
	Ok(Status {
		ipv4: Some(ipv4),
		ipv6,
		updated: Some(unix_now()),
		error: None,
	})
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		let mut forced = true;
		loop {
			if let Ok(settings) = load(&state.ubus).await
				&& settings.enabled
				&& !settings.hostname.is_empty()
				&& !settings.token.is_empty()
			{
				let status = match update(&state.ubus, &settings, forced).await {
					Ok(status) => status,
					Err(error) => Status {
						error: Some(error),
						..STATUS.lock().expect("ddns status").clone()
					},
				};
				*STATUS.lock().expect("ddns status") = status;
			}
			forced = tokio::time::timeout(INTERVAL, WAKE.notified()).await.is_ok();
		}
	});
}
