mod mdns;
mod vendor;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

pub use vendor::{maker, private_address};

use crate::networks::Layout;
use crate::platform::netlink::neighbors;
use crate::state::AppState;

const EVERY: Duration = Duration::from_secs(300);
const FIRST: Duration = Duration::from_secs(15);

static ANNOUNCED: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(Mutex::default);

pub fn announced(mac: &str) -> Option<String> {
	ANNOUNCED.lock().expect("discovery lock poisoned").get(mac).cloned()
}

fn readable(host: &str) -> Option<String> {
	let name = host.trim_end_matches('.').trim_end_matches(".local");
	if name.starts_with("Android_") {
		return Some("Android device".to_owned());
	}
	let name = name.replace('-', " ");
	(!name.trim().is_empty()).then(|| name.trim().to_owned())
}

async fn look_around(state: &AppState) {
	let Ok(layout) = Layout::load(&state.ubus).await else { return };
	let Ok(Ok(neighbors)) = tokio::task::spawn_blocking(neighbors::list).await else {
		return;
	};
	for network in layout.managed() {
		let (Some(device), Some(address)) = (
			layout.interface(&network.name).and_then(|i| i.get("device")),
			layout.address(&network.name),
		) else {
			continue;
		};
		let owners: HashMap<Ipv4Addr, String> = neighbors
			.iter()
			.filter(|n| n.device == device)
			.filter_map(|n| match n.ip {
				IpAddr::V4(ip) => Some((ip, n.mac.clone())),
				IpAddr::V6(_) => None,
			})
			.collect();
		if owners.is_empty() {
			continue;
		}
		let targets: Vec<Ipv4Addr> = owners.keys().copied().collect();
		let local = address.addr;
		let Ok(Ok(found)) = tokio::task::spawn_blocking(move || mdns::names(local, &targets)).await else {
			continue;
		};
		let mut announced = ANNOUNCED.lock().expect("discovery lock poisoned");
		for (ip, host) in found {
			if let (Some(mac), Some(name)) = (owners.get(&ip), readable(&host)) {
				announced.insert(mac.clone(), name);
			}
		}
	}
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		tokio::time::sleep(FIRST).await;
		loop {
			look_around(&state).await;
			tokio::time::sleep(EVERY).await;
		}
	});
}
