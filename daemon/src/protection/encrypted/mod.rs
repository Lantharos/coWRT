mod upstream;
mod wire;

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Notify;

use super::private_dns;
use crate::platform::dns;
use crate::system::events::{self, Tone};
use upstream::Upstream;

pub const PORT: u16 = 5053;
const FAILURES_BEFORE_FALLBACK: u32 = 2;
const RECHECK: Duration = Duration::from_secs(30);
const PROBE_NAME: &str = "example.com";

static FALLEN_BACK: AtomicBool = AtomicBool::new(false);
static RELOAD: Notify = Notify::const_new();

pub fn fallen_back() -> bool {
	FALLEN_BACK.load(Ordering::Relaxed)
}

pub fn reload() {
	RELOAD.notify_one();
}

struct Resolver {
	upstream: Upstream,
	plain: Vec<IpAddr>,
	failures: AtomicU32,
}

impl Resolver {
	async fn answer(&self, query: Bytes) -> Option<Bytes> {
		if !fallen_back() {
			match self.upstream.resolve(query.clone()).await {
				Ok(answer) => {
					self.failures.store(0, Ordering::Relaxed);
					return Some(answer);
				}
				Err(err) => self.failed(&err),
			}
		}
		wire::plain(&self.plain, &query).await
	}

	fn failed(&self, err: &io::Error) {
		let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
		if failures >= FAILURES_BEFORE_FALLBACK && !FALLEN_BACK.swap(true, Ordering::Relaxed) {
			events::record(
				Tone::Warn,
				format!("Encrypted DNS stopped answering ({err}), so lookups use regular DNS for now"),
			);
		}
	}

	async fn recheck(&self) {
		if !fallen_back() {
			return;
		}
		if self.upstream.resolve(Bytes::from(dns::question(0, PROBE_NAME))).await.is_ok() {
			self.failures.store(0, Ordering::Relaxed);
			FALLEN_BACK.store(false, Ordering::Relaxed);
			events::record(Tone::Good, "Encrypted DNS is answering again");
		}
	}
}

async fn serve_tcp(resolver: Arc<Resolver>, mut stream: TcpStream) -> io::Result<()> {
	loop {
		let length = stream.read_u16().await?;
		let mut query = vec![0u8; usize::from(length)];
		stream.read_exact(&mut query).await?;
		let Some(answer) = resolver.answer(Bytes::from(query)).await else {
			return Ok(());
		};
		stream.write_u16(answer.len() as u16).await?;
		stream.write_all(&answer).await?;
	}
}

async fn serve(resolver: Arc<Resolver>) -> io::Result<()> {
	let address = SocketAddr::from(([127, 0, 0, 1], PORT));
	let udp = Arc::new(UdpSocket::bind(address).await?);
	let tcp = TcpListener::bind(address).await?;
	let mut packet = [0u8; 4096];
	let mut recheck = tokio::time::interval(RECHECK);
	loop {
		tokio::select! {
			received = udp.recv_from(&mut packet) => {
				let (length, from) = received?;
				let query = Bytes::copy_from_slice(&packet[..length]);
				let (resolver, udp) = (resolver.clone(), udp.clone());
				tokio::spawn(async move {
					let Some(answer) = resolver.answer(query.clone()).await else { return };
					let reply = if answer.len() > wire::MAX_UDP { wire::truncated(&query).map(Bytes::from) } else { Some(answer) };
					if let Some(reply) = reply {
						let _ = udp.send_to(&reply, from).await;
					}
				});
			}
			accepted = tcp.accept() => {
				let (stream, _) = accepted?;
				tokio::spawn(serve_tcp(resolver.clone(), stream));
			}
			_ = recheck.tick() => {
				let resolver = resolver.clone();
				tokio::spawn(async move { resolver.recheck().await });
			}
			() = RELOAD.notified() => return Ok(()),
		}
	}
}

pub fn spawn(ubus: crate::platform::ubus::Ubus) {
	tokio::spawn(async move {
		let mut last_error = String::new();
		loop {
			let settings = private_dns::load(&ubus).await.unwrap_or_default();
			let Some((url, bootstrap)) = settings.resolver() else {
				FALLEN_BACK.store(false, Ordering::Relaxed);
				RELOAD.notified().await;
				continue;
			};
			let plain: Vec<IpAddr> = bootstrap.split(',').filter_map(|ip| ip.trim().parse().ok()).collect();
			let resolver = match Upstream::new(&url, &plain) {
				Ok(upstream) => Arc::new(Resolver {
					upstream,
					plain,
					failures: AtomicU32::new(0),
				}),
				Err(err) => {
					events::record(Tone::Bad, format!("Encrypted DNS couldn't start: {err}"));
					RELOAD.notified().await;
					continue;
				}
			};
			if let Err(err) = serve(resolver).await {
				let message = format!("Encrypted DNS stopped: {err}");
				if message != last_error {
					events::record(Tone::Bad, message.clone());
					last_error = message;
				}
				tokio::time::sleep(Duration::from_secs(5)).await;
			}
		}
	});
}
