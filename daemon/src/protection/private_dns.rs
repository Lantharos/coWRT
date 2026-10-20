use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::system::packages;

const SECTION: &str = "private_dns";
const PACKAGE: &str = "https-dns-proxy";
pub const SERVICE: &str = "/etc/init.d/https-dns-proxy";
pub const PORT: &str = "5053";

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
	#[default]
	Off,
	Cloudflare,
	Quad9,
	Google,
	Mullvad,
	Adguard,
	Nextdns,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub provider: Provider,
	pub nextdns: Option<String>,
}

impl Settings {
	pub fn resolver(&self) -> Option<(String, &'static str)> {
		let url = match self.provider {
			Provider::Off => return None,
			Provider::Cloudflare => ("https://cloudflare-dns.com/dns-query".into(), "1.1.1.1,1.0.0.1"),
			Provider::Quad9 => ("https://dns.quad9.net/dns-query".into(), "9.9.9.9,149.112.112.112"),
			Provider::Google => ("https://dns.google/dns-query".into(), "8.8.8.8,8.8.4.4"),
			Provider::Mullvad => ("https://dns.mullvad.net/dns-query".into(), "194.242.2.2,1.1.1.1"),
			Provider::Adguard => ("https://dns.adguard-dns.com/dns-query".into(), "94.140.14.14,94.140.15.15"),
			Provider::Nextdns => (
				format!("https://dns.nextdns.io/{}", self.nextdns.as_deref().unwrap_or_default()),
				"45.90.28.0,45.90.30.0",
			),
		};
		Some(url)
	}
}

pub async fn load(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let section = uci::sections_of(ubus, "cowrt", SECTION).await?.into_iter().next();
	Ok(section
		.map(|s| Settings {
			provider: serde_json::from_value(json!(s.get("provider").unwrap_or("off"))).unwrap_or_default(),
			nextdns: s.get("nextdns").map(str::to_owned),
		})
		.unwrap_or_default())
}

async fn service(action: &str) {
	let _ = Command::new(SERVICE).arg(action).status().await;
}

pub async fn store(ubus: &Ubus, settings: &Settings) -> Result<(), ApiError> {
	if settings.provider == Provider::Nextdns
		&& !settings
			.nextdns
			.as_deref()
			.is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
	{
		return Err(ApiError::bad_request("Paste your NextDNS configuration ID"));
	}
	super::guard::stand_down(ubus, &load(ubus).await?).await?;
	let mut values = json!({ "provider": serde_json::to_value(settings.provider).unwrap_or_default() });
	if let Some(id) = &settings.nextdns {
		values["nextdns"] = json!(id);
	}
	uci::upsert(ubus, "cowrt", SECTION, SECTION, values).await?;

	let Some((url, bootstrap)) = settings.resolver() else {
		if packages::installed(PACKAGE).await {
			service("stop").await;
			service("disable").await;
			forget_proxies(ubus).await?;
			let _ = Command::new("/etc/init.d/dnsmasq").arg("reload").status().await;
		}
		return Ok(());
	};
	packages::ensure(&[PACKAGE])
		.await
		.map_err(|err| ApiError::conflict(err.to_string()))?;
	service("stop").await;
	forget_proxies(ubus).await?;
	for section in uci::sections_of(ubus, PACKAGE, PACKAGE).await? {
		uci::delete(ubus, PACKAGE, &section.name).await?;
	}
	let resolver = json!({ "resolver_url": url, "bootstrap_dns": bootstrap, "listen_addr": "127.0.0.1", "listen_port": PORT });
	uci::add(ubus, PACKAGE, PACKAGE, None, resolver).await?;
	if let Some(main) = uci::sections_of(ubus, PACKAGE, "main").await?.into_iter().next() {
		uci::set(
			ubus,
			PACKAGE,
			&main.name,
			json!({ "dnsmasq_config_update": "*", "force_dns": "0" }),
		)
		.await?;
	}
	ubus.call("uci", "commit", json!({ "config": PACKAGE })).await?;
	service("enable").await;
	service("start").await;
	Ok(())
}

async fn forget_proxies(ubus: &Ubus) -> Result<(), ubus::Error> {
	for dnsmasq in uci::sections_of(ubus, "dhcp", "dnsmasq").await? {
		let servers: Vec<String> = dnsmasq
			.list("server")
			.into_iter()
			.filter(|s| !s.starts_with("127.0.0.1#505"))
			.collect();
		uci::unset(
			ubus,
			"dhcp",
			&dnsmasq.name,
			&["doh_backup_server", "doh_backup_noresolv", "server", "noresolv"],
		)
		.await?;
		if !servers.is_empty() {
			uci::set(ubus, "dhcp", &dnsmasq.name, json!({ "server": servers })).await?;
		}
	}
	ubus.call("uci", "commit", json!({ "config": "dhcp" })).await?;
	Ok(())
}
