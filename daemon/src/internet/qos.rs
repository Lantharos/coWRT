use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::devices::policy::MARK_MASK;
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};

const IFB: &str = "ifb-cowrt";
const SECTION: &str = "qos";

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
	#[default]
	Fixed,
	Speedtest,
	Adaptive,
}

impl Mode {
	fn parse(value: Option<&str>) -> Self {
		match value {
			Some("speedtest") => Self::Speedtest,
			Some("adaptive") => Self::Adaptive,
			_ => Self::Fixed,
		}
	}

	fn uci(self) -> &'static str {
		match self {
			Self::Fixed => "fixed",
			Self::Speedtest => "speedtest",
			Self::Adaptive => "adaptive",
		}
	}
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub enabled: bool,
	pub mode: Mode,
	pub download: u32,
	pub upload: u32,
}

impl Settings {
	pub fn shaping(&self) -> bool {
		self.enabled && self.download > 0 && self.upload > 0
	}
}

pub async fn load(ubus: &Ubus) -> Result<Settings, ubus::Error> {
	let sections = uci::sections_of(ubus, "cowrt", SECTION).await?;
	Ok(sections
		.first()
		.map(|section| Settings {
			enabled: section.flag("enabled"),
			mode: Mode::parse(section.get("mode")),
			download: section.get("download").and_then(|v| v.parse().ok()).unwrap_or_default(),
			upload: section.get("upload").and_then(|v| v.parse().ok()).unwrap_or_default(),
		})
		.unwrap_or_default())
}

pub async fn store(ubus: &Ubus, settings: &Settings) -> Result<(), ubus::Error> {
	let values = json!({
		"enabled": if settings.enabled { "1" } else { "0" },
		"mode": settings.mode.uci(),
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

pub async fn clear(wan: Option<&str>) {
	let Some(wan) = wan else { return };
	let _ = run("tc", &["qdisc", "del", "dev", wan, "root"]).await;
	let _ = run("tc", &["qdisc", "del", "dev", wan, "ingress"]).await;
	let _ = run("ip", &["link", "del", IFB]).await;
}

pub async fn apply(ubus: &Ubus, wan: Option<&str>) -> std::io::Result<()> {
	let Some(wan) = wan else { return Ok(()) };
	let settings = load(ubus).await.map_err(std::io::Error::other)?;
	clear(Some(wan)).await;
	if !settings.shaping() {
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

pub async fn set_bandwidth(wan: &str, download_kbit: u32, upload_kbit: u32) -> std::io::Result<()> {
	run(
		"tc",
		&[
			"qdisc",
			"change",
			"dev",
			wan,
			"root",
			"cake",
			"bandwidth",
			&format!("{upload_kbit}kbit"),
		],
	)
	.await?;
	run(
		"tc",
		&[
			"qdisc",
			"change",
			"dev",
			IFB,
			"root",
			"cake",
			"bandwidth",
			&format!("{download_kbit}kbit"),
		],
	)
	.await
}
