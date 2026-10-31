use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::watch;

use crate::platform::dns;
use crate::system::events::{self, Tone};
use crate::system::secret;

const PROBES: [[u8; 4]; 3] = [[1, 1, 1, 1], [9, 9, 9, 9], [8, 8, 8, 8]];
const PROBES6: [Ipv6Addr; 2] = [
	Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111),
	Ipv6Addr::new(0x2620, 0xfe, 0, 0, 0, 0, 0, 0xfe),
];
const PORT: u16 = 443;
const INTERVAL: Duration = Duration::from_secs(15);
const TIMEOUT: Duration = Duration::from_secs(3);
const RESOLVER: SocketAddr = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 53);
const MISSES_BEFORE_FAILING: u32 = 2;

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Internet {
	pub checked: bool,
	pub online: bool,
	pub latency_ms: Option<u32>,
	pub online6: bool,
	pub resolving: bool,
}

async fn probe(targets: impl IntoIterator<Item = IpAddr>) -> Option<u32> {
	for ip in targets {
		let started = Instant::now();
		if let Ok(Ok(_)) = tokio::time::timeout(TIMEOUT, TcpStream::connect(SocketAddr::new(ip, PORT))).await {
			return Some(started.elapsed().as_millis() as u32);
		}
	}
	None
}

async fn resolves() -> bool {
	let name = format!("{}.example.com", &secret::token()[..12]);
	dns::answers(RESOLVER, &name, TIMEOUT).await
}

pub fn spawn() -> watch::Receiver<Internet> {
	let (tx, rx) = watch::channel(Internet::default());
	tokio::spawn(async move {
		let mut misses = 0;
		let mut resolving = true;
		loop {
			let (latency_ms, latency6, answered) =
				tokio::join!(probe(PROBES.map(IpAddr::from)), probe(PROBES6.map(IpAddr::from)), resolves());
			let online = latency_ms.is_some();
			misses = if answered || !online { 0 } else { misses + 1 };
			let now_resolving = answered || !online || misses < MISSES_BEFORE_FAILING;
			if online && now_resolving != resolving {
				if now_resolving {
					events::record(Tone::Good, "Names are resolving again");
				} else {
					events::record(Tone::Bad, "The internet works, but names stopped resolving");
				}
			}
			resolving = now_resolving;
			tx.send_replace(Internet {
				checked: true,
				online,
				latency_ms,
				online6: latency6.is_some(),
				resolving,
			});
			tokio::time::sleep(INTERVAL).await;
		}
	});
	rx
}
