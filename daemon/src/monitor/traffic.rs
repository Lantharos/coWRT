use std::collections::HashMap;
use std::net::IpAddr;

use super::exact::Exact;
use crate::platform::netlink::conntrack::Flow;
use crate::platform::netlink::neighbors::Neighbor;

#[derive(Default)]
pub struct Accounting {
	seen: HashMap<(u8, u32), (u64, u64)>,
	exact_seen: HashMap<String, (u64, u64)>,
	primed: bool,
}

pub struct Sample {
	pub per_device: HashMap<String, (u64, u64)>,
}

impl Accounting {
	pub fn sample(&mut self, flows: Vec<Flow>, exact: Vec<Exact>, neighbors: &[Neighbor], wan_device: Option<&str>) -> Sample {
		let owners: HashMap<IpAddr, &str> = neighbors
			.iter()
			.filter(|n| Some(n.device.as_str()) != wan_device)
			.map(|n| (n.ip, n.mac.as_str()))
			.collect();
		let mut seen = HashMap::with_capacity(flows.len());
		let mut per_device: HashMap<String, (u64, u64)> = HashMap::new();
		for flow in flows {
			let key = (flow.family, flow.id);
			let (sent_before, received_before) =
				self.seen
					.get(&key)
					.copied()
					.unwrap_or(if self.primed { (0, 0) } else { (flow.sent, flow.received) });
			seen.insert(key, (flow.sent, flow.received));
			let sent = flow.sent.saturating_sub(sent_before);
			let received = flow.received.saturating_sub(received_before);
			if sent == 0 && received == 0 {
				continue;
			}
			let (mac, down, up) = match (owners.get(&flow.source), owners.get(&flow.destination)) {
				(Some(mac), _) => (*mac, received, sent),
				(None, Some(mac)) => (*mac, sent, received),
				(None, None) => continue,
			};
			let entry = per_device.entry(mac.to_owned()).or_default();
			entry.0 += down;
			entry.1 += up;
		}
		self.seen = seen;

		let mut exact_seen = HashMap::with_capacity(exact.len());
		for counter in exact {
			let (down_before, up_before) = match self.exact_seen.get(&counter.key) {
				Some(&(down, up)) if down <= counter.down && up <= counter.up => (down, up),
				Some(_) => (0, 0),
				None => (counter.down, counter.up),
			};
			exact_seen.insert(counter.key, (counter.down, counter.up));
			per_device.insert(counter.mac, (counter.down - down_before, counter.up - up_before));
		}
		self.exact_seen = exact_seen;
		self.primed = true;
		Sample { per_device }
	}
}
