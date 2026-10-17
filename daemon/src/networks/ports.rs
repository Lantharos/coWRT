use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use tokio::fs;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Port {
	pub name: String,
	pub label: String,
	pub up: bool,
	pub speed: Option<u32>,
	pub network: Option<String>,
	pub tagged: Vec<String>,
}

async fn read(path: impl AsRef<Path>) -> Option<String> {
	Some(fs::read_to_string(path).await.ok()?.trim().to_owned())
}

fn board_ports(board: &Value, role: &str) -> Vec<String> {
	let Some(entry) = board.pointer(&format!("/network/{role}")) else {
		return Vec::new();
	};
	match entry.get("ports").and_then(Value::as_array) {
		Some(ports) => ports.iter().filter_map(|p| p.as_str().map(str::to_owned)).collect(),
		None => entry
			.get("device")
			.and_then(Value::as_str)
			.map(str::to_owned)
			.into_iter()
			.collect(),
	}
}

async fn is_physical_ethernet(name: &str) -> bool {
	let dir = Path::new("/sys/class/net").join(name);
	dir.join("device").exists()
		&& !dir.join("phy80211").exists()
		&& !dir.join("dsa").exists()
		&& read(dir.join("type")).await.as_deref() == Some("1")
}

pub async fn detect(wan_devices: &[String], bridge_ports: &[String]) -> Vec<String> {
	let board: Value = fs::read_to_string("/etc/board.json")
		.await
		.ok()
		.and_then(|raw| serde_json::from_str(&raw).ok())
		.unwrap_or_default();
	let wan: BTreeSet<&String> = wan_devices.iter().collect();
	let mut ports: BTreeSet<String> = board_ports(&board, "lan").into_iter().collect();
	ports.extend(bridge_ports.iter().cloned());
	if let Ok(mut entries) = fs::read_dir("/sys/class/net").await {
		while let Ok(Some(entry)) = entries.next_entry().await {
			let name = entry.file_name().to_string_lossy().into_owned();
			if is_physical_ethernet(&name).await {
				ports.insert(name);
			}
		}
	}
	for role in ["wan", "wan6"] {
		for port in board_ports(&board, role) {
			ports.remove(&port);
		}
	}
	let mut ports: Vec<String> = ports.into_iter().filter(|p| !wan.contains(p)).collect();
	ports.sort_by_key(|name| natural_key(name));
	ports
}

fn natural_key(name: &str) -> (String, u32) {
	let digits = name.trim_start_matches(|c: char| !c.is_ascii_digit());
	(name[..name.len() - digits.len()].to_owned(), digits.parse().unwrap_or(0))
}

pub fn label(name: &str) -> String {
	match name.strip_prefix("lan") {
		Some(number) if number.chars().all(|c| c.is_ascii_digit()) && !number.is_empty() => format!("Port {number}"),
		_ => name.to_owned(),
	}
}

pub async fn link(name: &str) -> (bool, Option<u32>) {
	let dir = Path::new("/sys/class/net").join(name);
	let up = read(dir.join("carrier")).await.as_deref() == Some("1");
	let speed = if up {
		read(dir.join("speed"))
			.await
			.and_then(|s| s.parse().ok())
			.filter(|s: &u32| *s > 0)
	} else {
		None
	};
	(up, speed)
}
