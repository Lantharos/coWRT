use std::collections::HashMap;

use crate::devices::sources;
use crate::platform::ethtool;
use crate::platform::netlink::fdb;
use crate::platform::ubus::Ubus;

const PORT_COUNTERS: [[&str; 2]; 2] = [["TxBytes", "RxBytes"], ["TxByte", "RxGoodByte"]];

pub struct Exact {
	pub key: String,
	pub mac: String,
	pub down: u64,
	pub up: u64,
}

fn port_counters(port: &str) -> Option<(u64, u64)> {
	PORT_COUNTERS
		.iter()
		.find_map(|names| match ethtool::stats(port, names).ok()?.as_slice() {
			[Some(down), Some(up)] => Some((*down, *up)),
			_ => None,
		})
}

fn single_device_ports(stations: &[String]) -> Vec<(String, String)> {
	let Ok(learned) = fdb::learned() else { return Vec::new() };
	let mut ports: HashMap<String, Vec<String>> = HashMap::new();
	for entry in learned {
		if !stations.contains(&entry.mac) {
			ports.entry(entry.port).or_default().push(entry.mac);
		}
	}
	ports
		.into_iter()
		.filter_map(|(port, macs)| match macs.as_slice() {
			[mac] => Some((port, mac.clone())),
			_ => None,
		})
		.collect()
}

pub async fn gather(ubus: &Ubus) -> Vec<Exact> {
	let stations = sources::wifi_clients(ubus).await;
	let station_macs: Vec<String> = stations.iter().map(|s| s.mac.clone()).collect();
	let mut exact: Vec<Exact> = stations
		.into_iter()
		.map(|station| Exact {
			key: format!("station {}", station.mac),
			mac: station.mac,
			down: station.tx_bytes,
			up: station.rx_bytes,
		})
		.collect();
	let wired = tokio::task::spawn_blocking(move || {
		single_device_ports(&station_macs)
			.into_iter()
			.filter_map(|(port, mac)| {
				let (down, up) = port_counters(&port)?;
				Some(Exact {
					key: format!("port {port}"),
					mac,
					down,
					up,
				})
			})
			.collect::<Vec<_>>()
	})
	.await
	.unwrap_or_default();
	exact.extend(wired);
	exact
}
