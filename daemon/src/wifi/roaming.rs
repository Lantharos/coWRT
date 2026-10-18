use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::system::packages;

const SECTION: &str = "roaming";
const STEERING: &str = "usteer";
const FAST_OPTIONS: [&str; 6] = [
	"ieee80211r",
	"mobility_domain",
	"ft_over_ds",
	"ft_psk_generate_local",
	"ieee80211k",
	"bss_transition",
];

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Roaming {
	pub fast: bool,
	pub steering: bool,
}

pub async fn load(ubus: &Ubus) -> Result<Roaming, ubus::Error> {
	let sections = uci::sections_of(ubus, "cowrt", SECTION).await?;
	Ok(sections
		.first()
		.map(|s| Roaming {
			fast: s.flag("fast"),
			steering: s.flag("steering"),
		})
		.unwrap_or_default())
}

pub async fn store(ubus: &Ubus, roaming: Roaming) -> Result<(), ApiError> {
	let flag = |on: bool| if on { "1" } else { "0" };
	uci::upsert(
		ubus,
		"cowrt",
		SECTION,
		SECTION,
		json!({ "fast": flag(roaming.fast), "steering": flag(roaming.steering) }),
	)
	.await?;
	sync(ubus, roaming).await
}

fn mobility_domain(ssid: &str) -> String {
	let digest = Sha256::digest(ssid.as_bytes());
	format!("{:02x}{:02x}", digest[0], digest[1])
}

async fn full_wifi_stack() -> bool {
	packages::names().await.iter().any(|name| {
		(name.starts_with("wpad-") || name.starts_with("hostapd-"))
			&& !["basic", "mini", "common", "utils"].iter().any(|lite| name.contains(lite))
	})
}

fn uses_password(encryption: &str) -> bool {
	encryption.starts_with("psk") || encryption.starts_with("sae")
}

pub async fn sync(ubus: &Ubus, roaming: Roaming) -> Result<(), ApiError> {
	let neighbor_reports = roaming.fast && full_wifi_stack().await;
	for iface in uci::sections_of(ubus, "wireless", "wifi-iface").await? {
		if iface.get("mode").unwrap_or("ap") != "ap" {
			continue;
		}
		let encryption = iface.get("encryption").unwrap_or("none");
		if roaming.fast && uses_password(encryption) {
			let ssid = iface.get("ssid").unwrap_or_default();
			let values = json!({
				"ieee80211r": "1",
				"mobility_domain": mobility_domain(ssid),
				"ft_over_ds": "0",
				"ft_psk_generate_local": "1",
			});
			uci::set(ubus, "wireless", &iface.name, values).await?;
			if neighbor_reports {
				uci::set(
					ubus,
					"wireless",
					&iface.name,
					json!({ "ieee80211k": "1", "bss_transition": "1" }),
				)
				.await?;
			} else {
				uci::unset(ubus, "wireless", &iface.name, &["ieee80211k", "bss_transition"]).await?;
			}
		} else {
			uci::unset(ubus, "wireless", &iface.name, &FAST_OPTIONS).await?;
		}
	}
	Ok(())
}

async fn service(action: &str) {
	let _ = Command::new(format!("/etc/init.d/{STEERING}")).arg(action).output().await;
}

pub async fn steer(enabled: bool) -> Result<(), ApiError> {
	if enabled {
		packages::ensure(&[STEERING])
			.await
			.map_err(|err| ApiError::conflict(err.to_string()))?;
		service("enable").await;
		service("restart").await;
	} else if packages::installed(STEERING).await {
		service("stop").await;
		service("disable").await;
	}
	Ok(())
}
