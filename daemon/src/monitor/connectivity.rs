use std::net::SocketAddr;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::watch;

const PROBES: [([u8; 4], u16); 3] = [([1, 1, 1, 1], 443), ([9, 9, 9, 9], 443), ([8, 8, 8, 8], 443)];
const INTERVAL: Duration = Duration::from_secs(15);
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Internet {
	pub checked: bool,
	pub online: bool,
	pub latency_ms: Option<u32>,
}

async fn probe() -> Option<u32> {
	for (ip, port) in PROBES {
		let started = Instant::now();
		if let Ok(Ok(_)) = tokio::time::timeout(TIMEOUT, TcpStream::connect(SocketAddr::from((ip, port)))).await {
			return Some(started.elapsed().as_millis() as u32);
		}
	}
	None
}

pub fn spawn() -> watch::Receiver<Internet> {
	let (tx, rx) = watch::channel(Internet::default());
	tokio::spawn(async move {
		loop {
			let latency_ms = probe().await;
			tx.send_replace(Internet {
				checked: true,
				online: latency_ms.is_some(),
				latency_ms,
			});
			tokio::time::sleep(INTERVAL).await;
		}
	});
	rx
}
