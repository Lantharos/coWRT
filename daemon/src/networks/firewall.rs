use serde_json::json;

use crate::config::uci;
use crate::ubus::{self, Ubus};

const CONFIG: &str = "firewall";
const PREFIX: &str = "cw_";
pub const VPN_SERVER: &str = "vpn";
pub const TUNNEL: &str = "tunnel";

pub struct Policy {
	pub id: String,
	pub trusted: bool,
	pub internet: bool,
	pub tunneled: bool,
}

async fn policies(ubus: &Ubus) -> Result<Vec<Policy>, ubus::Error> {
	let cowrt = uci::sections(ubus, "cowrt").await?;
	let tunnel = cowrt.iter().find(|s| s.kind == "vpn_client").filter(|s| s.flag("enabled"));
	let tunneled: Vec<String> = tunnel.map(|s| s.list("networks")).unwrap_or_default();
	let mut policies: Vec<Policy> = cowrt
		.iter()
		.filter(|s| s.kind == "network")
		.map(|n| Policy {
			id: n.name.clone(),
			trusted: n.flag("trusted"),
			internet: n.flag("internet"),
			tunneled: tunneled.contains(&n.name),
		})
		.collect();
	if let Some(server) = cowrt.iter().find(|s| s.kind == "vpn_server").filter(|s| s.flag("enabled")) {
		let trusted = server.get("access") != Some("internet");
		policies.push(Policy {
			id: VPN_SERVER.to_owned(),
			trusted,
			internet: true,
			tunneled: false,
		});
	}
	Ok(policies)
}

pub async fn sync(ubus: &Ubus) -> Result<(), ubus::Error> {
	let policies = policies(ubus).await?;
	let sections = uci::sections(ubus, CONFIG).await?;
	let wan = sections
		.iter()
		.find(|s| s.kind == "zone" && s.list("network").iter().any(|n| n == "wan"))
		.and_then(|zone| zone.get("name"))
		.unwrap_or("wan")
		.to_owned();
	for section in sections.iter().filter(|s| s.name.starts_with(PREFIX)) {
		uci::delete(ubus, CONFIG, &section.name).await?;
	}

	if policies.iter().any(|p| p.tunneled) {
		let zone = json!({ "name": TUNNEL, "network": [TUNNEL], "input": "REJECT", "output": "ACCEPT", "forward": "REJECT", "masq": "1", "mtu_fix": "1" });
		uci::add(ubus, CONFIG, "zone", Some(&format!("{PREFIX}zone_{TUNNEL}")), zone).await?;
	}

	for policy in &policies {
		let id = &policy.id;
		let input = if policy.trusted { "ACCEPT" } else { "REJECT" };
		let zone = json!({ "name": id, "network": [id], "input": input, "output": "ACCEPT", "forward": "REJECT" });
		uci::add(ubus, CONFIG, "zone", Some(&format!("{PREFIX}zone_{id}")), zone).await?;

		if policy.internet {
			let dest = if policy.tunneled { TUNNEL } else { wan.as_str() };
			let forwarding = json!({ "src": id, "dest": dest });
			uci::add(ubus, CONFIG, "forwarding", Some(&format!("{PREFIX}fwd_{id}_out")), forwarding).await?;
		}
		if policy.trusted {
			for other in policies.iter().filter(|other| other.id != *id) {
				let forwarding = json!({ "src": id, "dest": other.id });
				uci::add(
					ubus,
					CONFIG,
					"forwarding",
					Some(&format!("{PREFIX}fwd_{id}_{}", other.id)),
					forwarding,
				)
				.await?;
			}
			continue;
		}
		let services = json!({
			"name": format!("Allow {id} DHCP and DNS"),
			"src": id,
			"proto": ["tcp", "udp"],
			"dest_port": ["53", "67", "68", "547", "5301", "5302"],
			"target": "ACCEPT",
		});
		uci::add(ubus, CONFIG, "rule", Some(&format!("{PREFIX}rule_{id}_services")), services).await?;
		let icmp = json!({ "name": format!("Allow {id} ICMP"), "src": id, "proto": "icmp", "target": "ACCEPT" });
		uci::add(ubus, CONFIG, "rule", Some(&format!("{PREFIX}rule_{id}_icmp")), icmp).await?;
	}
	Ok(())
}
