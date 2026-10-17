use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::config::uci;
use crate::policy::MARK_MASK;
use crate::ubus::{self, Ubus};

const IFB: &str = "ifb-cowrt";
const SECTION: &str = "qos";

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub enabled: bool,
	pub download: u32,
	pub upload: u32,
}

pub async fn load(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let sections = uci::sections_of(ubus, "cowrt", SECTION).await?;
	Ok(sections
		.first()
		.map(|section| Settings {
			enabled: section.flag("enabled"),
			download: section.get("download").and_then(|v| v.parse().ok()).unwrap_or_default(),
			upload: section.get("upload").and_then(|v| v.parse().ok()).unwrap_or_default(),
		})
		.unwrap_or_default())
}

pub async fn store(ubus: &Ubus, settings: &Settings) -> Result<(), ubus::Error> {
	let values = json!({
		"enabled": if settings.enabled { "1" } else { "0" },
		"download": settings.download.to_string(),
		"upload": settings.upload.to_string(),
	});
	uci::upsert(ubus, "cowrt", SECTION, SECTION, values).await
}

async fn run(program: &str, args: &[&str]) -> std::io::Result<()> {
	let output = Command::new(program).args(args).output().await?;
	if output.status.success() {
		return Ok(());
	}
	Err(std::io::Error::other(format!(
		"{program} {}: {}",
		args.join(" "),
		String::from_utf8_lossy(&output.stderr).trim()
	)))
}

async fn clear(wan: &str) {
	let _ = run("tc", &["qdisc", "del", "dev", wan, "root"]).await;
	let _ = run("tc", &["qdisc", "del", "dev", wan, "ingress"]).await;
	let _ = run("ip", &["link", "del", IFB]).await;
}

pub async fn apply(ubus: &Ubus, wan: Option<&str>) -> std::io::Result<()> {
	let Some(wan) = wan else { return Ok(()) };
	let settings = load(ubus).await.map_err(std::io::Error::other)?;
	clear(wan).await;
	if !settings.enabled || settings.download == 0 || settings.upload == 0 {
		return Ok(());
	}
	let mark = format!("{MARK_MASK:#x}");
	let upload = format!("{}kbit", settings.upload);
	let download = format!("{}kbit", settings.download);
	run("ip", &["link", "add", "name", IFB, "type", "ifb"]).await?;
	run("ip", &["link", "set", IFB, "up"]).await?;
	run(
		"tc",
		&[
			"qdisc",
			"replace",
			"dev",
			wan,
			"root",
			"cake",
			"bandwidth",
			&upload,
			"diffserv4",
			"dual-srchost",
			"nat",
			"ack-filter",
			"fwmark",
			&mark,
		],
	)
	.await?;
	run("tc", &["qdisc", "add", "dev", wan, "handle", "ffff:", "ingress"]).await?;
	run(
		"tc",
		&[
			"filter", "add", "dev", wan, "parent", "ffff:", "protocol", "all", "matchall", "action", "ctinfo", "cpmark", &mark,
			"action", "mirred", "egress", "redirect", "dev", IFB,
		],
	)
	.await?;
	run(
		"tc",
		&[
			"qdisc",
			"replace",
			"dev",
			IFB,
			"root",
			"cake",
			"bandwidth",
			&download,
			"diffserv4",
			"dual-dsthost",
			"nat",
			"wash",
			"ingress",
			"fwmark",
			&mark,
		],
	)
	.await
}
