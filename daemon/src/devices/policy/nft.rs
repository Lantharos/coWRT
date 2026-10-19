use std::fmt::Write;
use std::path::Path;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::{MARK_MASK, Policies};

pub fn device_exists(name: &str) -> bool {
	Path::new("/sys/class/net").join(name).exists()
}

fn quoted(names: &[String]) -> String {
	let names: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
	format!("{{ {} }}", names.join(", "))
}

fn kbytes(kbit: u32) -> u32 {
	(kbit / 8).max(1)
}

pub fn render(policies: &Policies, devices: &[String], redirects: &[(String, u16)]) -> String {
	let mut out = String::from("table inet cowrt\ndelete table inet cowrt\ntable netdev cowrt\ndelete table netdev cowrt\n");
	let leaves_home = if devices.is_empty() {
		String::new()
	} else {
		format!(" oifname != {}", quoted(devices))
	};

	out.push_str("table inet cowrt {\n\tchain forward {\n\t\ttype filter hook forward priority filter - 5; policy accept;\n");
	for policy in &policies.devices {
		let mac = &policy.mac;
		let controls = &policy.controls;
		if controls.blocked {
			let _ = writeln!(out, "\t\tether saddr {mac}{leaves_home} drop");
		}
		for schedule in &controls.schedules {
			for window in schedule.nft_windows() {
				let _ = writeln!(out, "\t\tether saddr {mac} {window}{leaves_home} drop");
			}
		}
		if let Some(mark) = controls.priority.mark() {
			let _ = writeln!(
				out,
				"\t\tether saddr {mac} ct state new ct mark set ct mark & {:#x} | {mark:#x}",
				!MARK_MASK
			);
		}
	}
	let _ = writeln!(out, "\t\tct mark & {MARK_MASK:#x} != 0 meta mark set ct mark");
	out.push_str("\t}\n");
	if !redirects.is_empty() {
		out.push_str("\tchain dns {\n\t\ttype nat hook prerouting priority dstnat - 5; policy accept;\n");
		for (device, port) in redirects {
			let _ = writeln!(
				out,
				"\t\tiifname \"{device}\" meta l4proto {{ tcp, udp }} th dport 53 redirect to :{port}"
			);
		}
		out.push_str("\t}\n");
	}
	out.push_str("}\n");

	let limits: Vec<(String, Option<u32>, Option<u32>)> = policies
		.devices
		.iter()
		.map(|p| (p.mac.clone(), p.controls.limit_down, p.controls.limit_up))
		.chain(
			policies
				.shared
				.iter()
				.map(|shared| (macs(&shared.macs), shared.down, shared.up)),
		)
		.filter(|(_, down, up)| down.is_some() || up.is_some())
		.collect();
	if limits.is_empty() || devices.is_empty() {
		return out;
	}
	let hooked = quoted(devices);
	let _ = writeln!(
		out,
		"table netdev cowrt {{\n\tchain ingress {{\n\t\ttype filter hook ingress devices = {hooked} priority -500; policy accept;"
	);
	for (macs, _, up) in &limits {
		if let Some(up) = up {
			limit_rule(&mut out, "saddr", macs, *up);
		}
	}
	let _ = writeln!(
		out,
		"\t}}\n\tchain egress {{\n\t\ttype filter hook egress devices = {hooked} priority -500; policy accept;"
	);
	for (macs, down, _) in &limits {
		if let Some(down) = down {
			limit_rule(&mut out, "daddr", macs, *down);
		}
	}
	out.push_str("\t}\n}\n");
	out
}

fn macs(list: &[String]) -> String {
	format!("{{ {} }}", list.join(", "))
}

fn limit_rule(out: &mut String, direction: &str, macs: &str, kbit: u32) {
	let rate = kbytes(kbit);
	let _ = writeln!(
		out,
		"\t\tether {direction} {macs} limit rate over {rate} kbytes/second burst {} kbytes drop",
		rate / 4 + 16
	);
}

pub async fn run(ruleset: &str) -> std::io::Result<()> {
	let timezone = tokio::fs::read_to_string("/etc/TZ").await.unwrap_or_default();
	let mut child = Command::new("nft")
		.args(["-f", "-"])
		.env("TZ", timezone.trim())
		.stdin(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;
	child.stdin.take().expect("piped stdin").write_all(ruleset.as_bytes()).await?;
	let output = child.wait_with_output().await?;
	if output.status.success() {
		return Ok(());
	}
	Err(std::io::Error::other(
		String::from_utf8_lossy(&output.stderr).trim().to_owned(),
	))
}
