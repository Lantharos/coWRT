use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::json;
use tokio::net::UdpSocket;
use tokio::process::Command;

use super::private_dns::{self, PORT, SERVICE};
use crate::platform::config::uci;
use crate::platform::ubus::{self, Ubus};
use crate::state::AppState;

const EVERY: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(3);
const RESTART_AFTER: u32 = 2;
const FALL_BACK_AFTER: u32 = 3;
const RESTART_EVERY: u32 = 6;
const PROBE_NAME: &str = "openwrt.org";

static FALLEN_BACK: AtomicBool = AtomicBool::new(false);

pub fn fallen_back() -> bool {
	FALLEN_BACK.load(Ordering::Relaxed)
}

fn query(id: u16) -> Vec<u8> {
	let mut packet = Vec::with_capacity(32);
	packet.extend_from_slice(&id.to_be_bytes());
	packet.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
	for label in PROBE_NAME.split('.') {
		packet.push(label.len() as u8);
		packet.extend_from_slice(label.as_bytes());
	}
	packet.extend_from_slice(&[0, 0, 1, 0, 1]);
	packet
}

async fn answers() -> bool {
	let probe = async {
		let socket = UdpSocket::bind("127.0.0.1:0").await.ok()?;
		let id = std::process::id() as u16 ^ 0x5a17;
		let proxy: SocketAddr = format!("127.0.0.1:{PORT}").parse().ok()?;
		socket.send_to(&query(id), proxy).await.ok()?;
		let mut buffer = [0u8; 512];
		let length = socket.recv(&mut buffer).await.ok()?;
		(length >= 2 && buffer[..2] == id.to_be_bytes()).then_some(())
	};
	tokio::time::timeout(TIMEOUT, probe).await.ok().flatten().is_some()
}

fn plain_servers(bootstrap: &str) -> Vec<String> {
	bootstrap.split(',').map(str::to_owned).collect()
}

async fn set_plain(ubus: &Ubus, servers: &[String], enabled: bool) -> Result<(), ubus::Error> {
	for dnsmasq in uci::sections_of(ubus, "dhcp", "dnsmasq").await? {
		let mut list: Vec<String> = dnsmasq.list("server").into_iter().filter(|s| !servers.contains(s)).collect();
		if enabled {
			list.extend(servers.iter().cloned());
		}
		uci::set(ubus, "dhcp", &dnsmasq.name, json!({ "server": list })).await?;
	}
	ubus.call("uci", "commit", json!({ "config": "dhcp" })).await?;
	let _ = Command::new("/etc/init.d/dnsmasq").arg("reload").status().await;
	Ok(())
}

pub async fn stand_down(ubus: &Ubus, settings: &private_dns::Settings) -> Result<(), ubus::Error> {
	let Some((_, bootstrap)) = settings.resolver() else {
		return Ok(());
	};
	let servers = plain_servers(bootstrap);
	let leftover = uci::sections_of(ubus, "dhcp", "dnsmasq")
		.await?
		.iter()
		.any(|dnsmasq| dnsmasq.list("server").iter().any(|s| servers.contains(s)));
	if leftover {
		set_plain(ubus, &servers, false).await?;
	}
	FALLEN_BACK.store(false, Ordering::Relaxed);
	Ok(())
}

async fn restart_proxy() {
	let _ = Command::new(SERVICE).arg("restart").status().await;
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		if let Ok(settings) = private_dns::load(&state.ubus).await {
			let _ = stand_down(&state.ubus, &settings).await;
		}
		let mut failures = 0u32;
		loop {
			tokio::time::sleep(EVERY).await;
			let Some((_, bootstrap)) = private_dns::load(&state.ubus).await.ok().and_then(|s| s.resolver()) else {
				failures = 0;
				continue;
			};
			let servers = plain_servers(bootstrap);
			if answers().await {
				failures = 0;
				if fallen_back() && set_plain(&state.ubus, &servers, false).await.is_ok() {
					FALLEN_BACK.store(false, Ordering::Relaxed);
					eprintln!("cowrtd: encrypted DNS is answering again");
				}
				continue;
			}
			failures += 1;
			if failures == RESTART_AFTER || (failures > RESTART_AFTER && failures.is_multiple_of(RESTART_EVERY)) {
				eprintln!("cowrtd: encrypted DNS stopped answering, restarting it");
				restart_proxy().await;
			}
			if failures >= FALL_BACK_AFTER && !fallen_back() && set_plain(&state.ubus, &servers, true).await.is_ok() {
				FALLEN_BACK.store(true, Ordering::Relaxed);
				eprintln!("cowrtd: using regular DNS until encrypted DNS answers again");
			}
		}
	});
}
