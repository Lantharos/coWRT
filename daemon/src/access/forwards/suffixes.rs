use std::net::{IpAddr, Ipv6Addr};

use serde_json::json;

use crate::platform::config::uci;
use crate::platform::netlink::neighbors::{self, Neighbor};
use crate::state::AppState;

const KEPT: usize = 4;
const HOST_BITS: u128 = u64::MAX as u128;

fn suffix(ip: Ipv6Addr) -> String {
	format!("{}/::ffff:ffff:ffff:ffff", Ipv6Addr::from(u128::from(ip) & HOST_BITS))
}

fn global_suffixes(neighbors: &[Neighbor], mac: &str) -> Vec<String> {
	neighbors
		.iter()
		.filter(|n| n.mac == mac)
		.filter_map(|n| match n.ip {
			IpAddr::V6(ip) if ip.segments()[0] & 0xe000 == 0x2000 => Some(suffix(ip)),
			_ => None,
		})
		.collect()
}

pub async fn learn(state: &AppState) {
	let Ok(Ok(neighbors)) = tokio::task::spawn_blocking(neighbors::list).await else {
		return;
	};
	let Ok(forwards) = uci::sections_of(&state.ubus, "cowrt", "forward").await else {
		return;
	};
	let mut updates = Vec::new();
	for forward in forwards.iter().filter(|f| !f.flag("disabled")) {
		let Some(mac) = forward.get("mac") else { continue };
		let known = forward.list("suffix6");
		let mut merged = global_suffixes(&neighbors, &mac.to_ascii_lowercase());
		merged.extend(known.iter().cloned());
		let mut seen = std::collections::HashSet::new();
		merged.retain(|entry| seen.insert(entry.clone()));
		merged.truncate(KEPT);
		if merged != known {
			updates.push((forward.name.clone(), merged));
		}
	}
	if updates.is_empty() {
		return;
	}
	let _ = state
		.applier
		.apply(&state.ubus, &["cowrt", "firewall"], None, async |ubus| {
			for (section, suffixes) in &updates {
				uci::set(ubus, "cowrt", section, json!({ "suffix6": suffixes })).await?;
			}
			super::sync(ubus).await?;
			Ok(())
		})
		.await;
}
