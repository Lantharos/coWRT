use std::io;
use tokio::process::Command;

use crate::platform::nft;

const TABLE: &str = "4242";
const MAIN_PRIORITY: &str = "30000";
const TABLE_PRIORITY: &str = "30001";
const NAT_TABLE: &str = "cowrt_uplink";

#[derive(Clone, Copy)]
pub enum Family {
	V4,
	V6,
}

impl Family {
	fn flag(self) -> &'static str {
		match self {
			Self::V4 => "-4",
			Self::V6 => "-6",
		}
	}
}

async fn ip(family: Family, args: &[&str]) -> io::Result<String> {
	let output = Command::new("ip").arg(family.flag()).args(args).output().await?;
	if output.status.success() {
		return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
	}
	Err(io::Error::other(String::from_utf8_lossy(&output.stderr).trim().to_owned()))
}

async fn ensure_rule(family: Family, priority: &str, rule: &[&str]) -> io::Result<()> {
	if !ip(family, &["rule", "show", "pref", priority]).await?.trim().is_empty() {
		return Ok(());
	}
	let mut args = vec!["rule", "add", "pref", priority];
	args.extend_from_slice(rule);
	ip(family, &args).await.map(drop)
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

pub async fn steer(family: Family, route: &Route<'_>) -> io::Result<()> {
	ensure_rule(family, MAIN_PRIORITY, &["lookup", "main", "suppress_prefixlength", "0"]).await?;
	ensure_rule(family, TABLE_PRIORITY, &["lookup", TABLE]).await?;
	let current = ip(family, &["route", "show", "table", TABLE]).await.unwrap_or_default();
	let target = wanted(route);
	if current.lines().any(|line| line.trim_start().starts_with(&target)) {
		return Ok(());
	}
	let mut args = vec!["route", "replace", "default"];
	if let Some(gateway) = route.gateway {
		args.extend(["via", gateway]);
	}
	args.extend(["dev", route.device, "table", TABLE]);
	ip(family, &args).await.map(drop)
}

pub async fn release(family: Family) {
	let _ = ip(family, &["rule", "del", "pref", TABLE_PRIORITY]).await;
	let _ = ip(family, &["rule", "del", "pref", MAIN_PRIORITY]).await;
	let _ = ip(family, &["route", "flush", "table", TABLE]).await;
}

pub enum Ipv6Rules<'a> {
	None,
	Masquerade(&'a str),
	Refuse(&'a str),
}

const LINK_SCOPED: &str = "{ fe80::/10, ff00::/8 }";

fn refuse_chain(name: &str, hook: &str, device: &str) -> String {
	format!(
		"\tchain {name} {{\n\t\ttype filter hook {hook} priority filter - 10; policy accept;\n\t\toifname \"{device}\" ip6 daddr != {LINK_SCOPED} meta l4proto tcp reject with tcp reset\n\t\toifname \"{device}\" ip6 daddr != {LINK_SCOPED} reject with icmpv6 admin-prohibited\n\t}}\n"
	)
}

pub async fn ipv6_rules(rules: Ipv6Rules<'_>) -> io::Result<()> {
	let mut ruleset = format!("table ip6 {NAT_TABLE}\ndelete table ip6 {NAT_TABLE}\n");
	match rules {
		Ipv6Rules::None => {}
		Ipv6Rules::Masquerade(device) => ruleset.push_str(&format!(
			"table ip6 {NAT_TABLE} {{\n\tchain postrouting {{\n\t\ttype nat hook postrouting priority srcnat; policy accept;\n\t\toifname \"{device}\" masquerade\n\t}}\n}}\n"
		)),
		Ipv6Rules::Refuse(device) => ruleset.push_str(&format!(
			"table ip6 {NAT_TABLE} {{\n{}{}}}\n",
			refuse_chain("forward", "forward", device),
			refuse_chain("output", "output", device)
		)),
	}
	nft::run(&ruleset).await
}
