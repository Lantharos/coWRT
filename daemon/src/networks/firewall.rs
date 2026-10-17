use serde_json::json;

use crate::config::uci;
use crate::ubus::{self, Ubus};

const CONFIG: &str = "firewall";
const PREFIX: &str = "cw_";

pub struct Policy {
	pub id: String,
	pub trusted: bool,
	pub internet: bool,
}

pub async fn sync(ubus: &Ubus, policies: &[Policy]) -> Result<(), ubus::Error> {
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

	for policy in policies {
		let id = &policy.id;
		let input = if policy.trusted { "ACCEPT" } else { "REJECT" };
		let zone = json!({ "name": id, "network": [id], "input": input, "output": "ACCEPT", "forward": "REJECT" });
		uci::add(ubus, CONFIG, "zone", Some(&format!("{PREFIX}zone_{id}")), zone).await?;

		if policy.internet {
			let forwarding = json!({ "src": id, "dest": wan });
			uci::add(ubus, CONFIG, "forwarding", Some(&format!("{PREFIX}fwd_{id}_wan")), forwarding).await?;
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
			"dest_port": ["53", "67", "68", "547"],
			"target": "ACCEPT",
		});
		uci::add(ubus, CONFIG, "rule", Some(&format!("{PREFIX}rule_{id}_services")), services).await?;
		let icmp = json!({ "name": format!("Allow {id} ICMP"), "src": id, "proto": "icmp", "target": "ACCEPT" });
		uci::add(ubus, CONFIG, "rule", Some(&format!("{PREFIX}rule_{id}_icmp")), icmp).await?;
	}
	Ok(())
}
