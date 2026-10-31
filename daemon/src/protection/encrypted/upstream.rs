use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use h2::client::SendRequest;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, crypto};

const CONNECT: Duration = Duration::from_secs(3);
const QUERY: Duration = Duration::from_secs(3);
const MAX_ANSWER: usize = 65_535;
const MEDIA_TYPE: &str = "application/dns-message";

pub struct Upstream {
	host: String,
	uri: String,
	addresses: Vec<SocketAddr>,
	tls: TlsConnector,
	sender: Mutex<Option<SendRequest<Bytes>>>,
}

fn failed(err: impl std::fmt::Display) -> io::Error {
	io::Error::other(err.to_string())
}

impl Upstream {
	pub fn new(url: &str, addresses: &[IpAddr]) -> io::Result<Self> {
		let host = url
			.strip_prefix("https://")
			.and_then(|rest| rest.split('/').next())
			.ok_or_else(|| failed("the DNS address isn't an https:// URL"))?
			.to_owned();
		let roots = RootCertStore {
			roots: webpki_roots::TLS_SERVER_ROOTS.into(),
		};
		let mut config = ClientConfig::builder_with_provider(Arc::new(crypto::ring::default_provider()))
			.with_safe_default_protocol_versions()
			.map_err(failed)?
			.with_root_certificates(roots)
			.with_no_client_auth();
		config.alpn_protocols = vec![b"h2".to_vec()];
		Ok(Self {
			host,
			uri: url.to_owned(),
			addresses: addresses.iter().map(|ip| SocketAddr::new(*ip, 443)).collect(),
			tls: TlsConnector::from(Arc::new(config)),
			sender: Mutex::default(),
		})
	}

	async fn connect(&self) -> io::Result<SendRequest<Bytes>> {
		let name = ServerName::try_from(self.host.clone()).map_err(failed)?;
		for address in &self.addresses {
			let Ok(Ok(tcp)) = timeout(CONNECT, TcpStream::connect(address)).await else {
				continue;
			};
			tcp.set_nodelay(true)?;
			let Ok(Ok(tls)) = timeout(CONNECT, self.tls.connect(name.clone(), tcp)).await else {
				continue;
			};
			let (sender, connection) = h2::client::handshake(tls).await.map_err(failed)?;
			tokio::spawn(async move {
				let _ = connection.await;
			});
			return Ok(sender);
		}
		Err(failed("none of the DNS server's addresses answered"))
	}

	async fn sender(&self) -> io::Result<SendRequest<Bytes>> {
		let mut slot = self.sender.lock().await;
		if let Some(sender) = slot.as_ref() {
			return Ok(sender.clone());
		}
		let sender = self.connect().await?;
		*slot = Some(sender.clone());
		Ok(sender)
	}

	async fn exchange(&self, query: Bytes) -> io::Result<Bytes> {
		let mut sender = self.sender().await?.ready().await.map_err(failed)?;
		let request = http::Request::post(&self.uri)
			.header("content-length", query.len())
			.header("content-type", MEDIA_TYPE)
			.header("accept", MEDIA_TYPE)
			.body(())
			.map_err(failed)?;
		let (response, mut stream) = sender.send_request(request, false).map_err(failed)?;
		stream.send_data(query, true).map_err(failed)?;
		let response = response.await.map_err(failed)?;
		if !response.status().is_success() {
			return Err(failed(format!("the DNS server answered {}", response.status())));
		}
		let mut body = response.into_body();
		let mut answer = BytesMut::new();
		while let Some(chunk) = body.data().await {
			let chunk = chunk.map_err(failed)?;
			let _ = body.flow_control().release_capacity(chunk.len());
			answer.extend_from_slice(&chunk);
			if answer.len() > MAX_ANSWER {
				return Err(failed("the DNS answer was too large"));
			}
		}
		Ok(answer.freeze())
	}

	pub async fn resolve(&self, query: Bytes) -> io::Result<Bytes> {
		match timeout(QUERY, self.exchange(query.clone())).await {
			Ok(Ok(answer)) => return Ok(answer),
			Ok(Err(_)) => {}
			Err(_) => {
				*self.sender.lock().await = None;
				return Err(failed("the DNS server took too long"));
			}
		}
		*self.sender.lock().await = None;
		timeout(QUERY, self.exchange(query))
			.await
			.map_err(|_| failed("the DNS server took too long"))?
	}
}
