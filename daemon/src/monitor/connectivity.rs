use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::watch;

const PROBES: [[u8; 4]; 3] = [[1, 1, 1, 1], [9, 9, 9, 9], [8, 8, 8, 8]];
const PROBES6: [Ipv6Addr; 2] = [
	Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111),
	Ipv6Addr::new(0x2620, 0xfe, 0, 0, 0, 0, 0, 0xfe),
];
const PORT: u16 = 443;
const INTERVAL: Duration = Duration::from_secs(15);
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Internet {
	pub checked: bool,
	pub online: bool,
	pub latency_ms: Option<u32>,
	pub online6: bool,
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

pub fn spawn() -> watch::Receiver<Internet> {
	let (tx, rx) = watch::channel(Internet::default());
	tokio::spawn(async move {
		loop {
			let (latency_ms, latency6) = tokio::join!(probe(PROBES.map(IpAddr::from)), probe(PROBES6.map(IpAddr::from)));
			tx.send_replace(Internet {
				checked: true,
				online: latency_ms.is_some(),
				latency_ms,
				online6: latency6.is_some(),
			});
			tokio::time::sleep(INTERVAL).await;
		}
	});
	rx
}
