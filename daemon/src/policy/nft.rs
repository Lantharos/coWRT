use std::fmt::Write;
use std::path::Path;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::{DevicePolicy, MARK_MASK};

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

pub fn render(policies: &[DevicePolicy], devices: &[String]) -> String {
	let mut out = String::from("table inet cowrt\ndelete table inet cowrt\ntable netdev cowrt\ndelete table netdev cowrt\n");
	let leaves_home = if devices.is_empty() {
		String::new()
	} else {
		format!(" oifname != {}", quoted(devices))
	};

	out.push_str("table inet cowrt {\n\tchain forward {\n\t\ttype filter hook forward priority filter - 5; policy accept;\n");
	for policy in policies {
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
	out.push_str("\t}\n}\n");

	let limited: Vec<&DevicePolicy> = policies
		.iter()
		.filter(|p| p.controls.limit_down.is_some() || p.controls.limit_up.is_some())
		.collect();
	if limited.is_empty() || devices.is_empty() {
		return out;
	}
	let hooked = quoted(devices);
	let _ = writeln!(
		out,
		"table netdev cowrt {{\n\tchain ingress {{\n\t\ttype filter hook ingress devices = {hooked} priority -500; policy accept;"
	);
	for policy in &limited {
		if let Some(up) = policy.controls.limit_up {
			let rate = kbytes(up);
			let _ = writeln!(
				out,
				"\t\tether saddr {} limit rate over {rate} kbytes/second burst {} kbytes drop",
				policy.mac,
				rate / 4 + 16
			);
		}
	}
	let _ = writeln!(
		out,
		"\t}}\n\tchain egress {{\n\t\ttype filter hook egress devices = {hooked} priority -500; policy accept;"
	);
	for policy in &limited {
		if let Some(down) = policy.controls.limit_down {
			let rate = kbytes(down);
			let _ = writeln!(
				out,
				"\t\tether daddr {} limit rate over {rate} kbytes/second burst {} kbytes drop",
				policy.mac,
				rate / 4 + 16
			);
		}
	}
	out.push_str("\t}\n}\n");
	out
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
