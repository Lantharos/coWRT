use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use super::tether;
use crate::http::ApiError;
use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};
use crate::state::AppState;

const SETTINGS: &str = "failover";
pub const PRIMARY: &str = "wan";
pub const AUTO: &str = "auto";
const UPLINK_PROTOS: [&str; 6] = ["dhcp", "static", "pppoe", "ncm", "qmi", "mbim"];
const PHONE_HINTS: [&str; 4] = ["phone", "mobile", "usb", "tether"];

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
	Cable,
	Phone,
	Other,
}

impl Kind {
	fn parse(value: Option<&str>) -> Self {
		match value {
			Some("cable") => Self::Cable,
			Some("phone") => Self::Phone,
			_ => Self::Other,
		}
	}

	fn uci(self) -> &'static str {
		match self {
			Self::Cable => "cable",
			Self::Phone => "phone",
			Self::Other => "other",
		}
	}
}

#[derive(Deserialize)]
pub struct Settings {
	pub mode: String,
	pub failback: bool,
}

pub struct Configured {
	pub id: String,
	pub name: String,
	pub kind: Kind,
}

pub async fn configured(ubus: &Ubus) -> Result<Vec<Configured>, ubus::Error> {
	Ok(uci::sections_of(ubus, "cowrt", "uplink")
		.await?
		.into_iter()
		.map(|s| Configured {
			name: s.get("name").unwrap_or(&s.name).to_owned(),
			kind: Kind::parse(s.get("kind")),
			id: s.name,
		})
		.collect())
}

pub async fn settings(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", SETTINGS).await?.into_iter().next();
	Ok(Settings {
		mode: section.as_ref().and_then(|s| s.get("mode")).unwrap_or(AUTO).to_owned(),
		failback: section.is_none_or(|s| s.get("failback") != Some("0")),
	})
}

pub async fn store_settings(ubus: &Ubus, input: &Settings) -> Result<(), ubus::Error> {
	let values = json!({ "mode": input.mode, "failback": if input.failback { "1" } else { "0" } });
	uci::upsert(ubus, "cowrt", SETTINGS, SETTINGS, values).await
}

pub async fn add(ubus: &Ubus, id: &str, name: &str, kind: Kind) -> Result<(), ubus::Error> {
	uci::upsert(ubus, "cowrt", "uplink", id, json!({ "name": name, "kind": kind.uci() })).await
}

pub async fn forget(ubus: &Ubus, id: &str) -> Result<(), ubus::Error> {
	uci::delete(ubus, "cowrt", id).await
}

fn masqueraded(firewall: &[Section]) -> Vec<String> {
	firewall
		.iter()
		.filter(|s| s.kind == "zone" && s.flag("masq"))
		.flat_map(|s| s.list("network"))
		.collect()
}

pub async fn retire_mwan3() {
	let script = "/etc/init.d/mwan3";
	if tokio::fs::metadata(script).await.is_err() {
		return;
	}
	for action in ["stop", "disable"] {
		let _ = Command::new(script).arg(action).output().await;
	}
}

pub async fn import(state: &AppState) -> Result<(), ApiError> {
	if !configured(&state.ubus).await?.is_empty() {
		return Ok(());
	}
	let (network, firewall) = tokio::try_join!(uci::sections(&state.ubus, "network"), uci::sections(&state.ubus, "firewall"))?;
	let outside = masqueraded(&firewall);
	let mut found = vec![(PRIMARY.to_owned(), "Main line".to_owned(), Kind::Cable)];
	for interface in network.iter().filter(|s| s.kind == "interface" && s.name != PRIMARY) {
		let proto = interface.get("proto").unwrap_or_default();
		if !outside.contains(&interface.name) || !UPLINK_PROTOS.contains(&proto) {
			continue;
		}
		let device = interface.get("device").unwrap_or_default();
		let phone = PHONE_HINTS.iter().any(|hint| interface.name.contains(hint)) || tether::is_tether(device).await;
		let (name, kind) = if phone {
			("Phone".to_owned(), Kind::Phone)
		} else {
			(interface.name.clone(), Kind::Other)
		};
		found.push((interface.name.clone(), name, kind));
	}
	let several = found.len() > 1;
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			for (id, name, kind) in &found {
				add(ubus, id, name, *kind).await?;
			}
			Ok(())
		})
		.await?;
	if several {
		retire_mwan3().await;
	}
	Ok(())
}
