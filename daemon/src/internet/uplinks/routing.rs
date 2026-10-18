use std::io;

use tokio::process::Command;

const TABLE: &str = "4242";
const MAIN_PRIORITY: &str = "30000";
const TABLE_PRIORITY: &str = "30001";

async fn ip(args: &[&str]) -> io::Result<String> {
	let output = Command::new("ip").args(args).output().await?;
	if output.status.success() {
		return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
	}
	Err(io::Error::other(String::from_utf8_lossy(&output.stderr).trim().to_owned()))
}

async fn ensure_rule(priority: &str, rule: &[&str]) -> io::Result<()> {
	if !ip(&["-4", "rule", "show", "pref", priority]).await?.trim().is_empty() {
		return Ok(());
	}
	let mut args = vec!["-4", "rule", "add", "pref", priority];
	args.extend_from_slice(rule);
	ip(&args).await.map(drop)
}

pub struct Route<'a> {
	pub device: &'a str,
	pub gateway: Option<&'a str>,
}

fn wanted(route: &Route) -> String {
	match route.gateway {
		Some(gateway) => format!("default via {gateway} dev {}", route.device),
		None => format!("default dev {}", route.device),
	}
}

pub async fn steer(route: &Route<'_>) -> io::Result<()> {
	ensure_rule(MAIN_PRIORITY, &["lookup", "main", "suppress_prefixlength", "0"]).await?;
	ensure_rule(TABLE_PRIORITY, &["lookup", TABLE]).await?;
	let current = ip(&["-4", "route", "show", "table", TABLE]).await.unwrap_or_default();
	let target = wanted(route);
	if current.lines().any(|line| line.trim_start().starts_with(&target)) {
		return Ok(());
	}
	let mut args = vec!["-4", "route", "replace", "default"];
	if let Some(gateway) = route.gateway {
		args.extend(["via", gateway]);
	}
	args.extend(["dev", route.device, "table", TABLE]);
	ip(&args).await.map(drop)
}

pub async fn release() {
	let _ = ip(&["-4", "rule", "del", "pref", TABLE_PRIORITY]).await;
	let _ = ip(&["-4", "rule", "del", "pref", MAIN_PRIORITY]).await;
	let _ = ip(&["-4", "route", "flush", "table", TABLE]).await;
}
