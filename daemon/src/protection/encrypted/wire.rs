use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use bytes::Bytes;
use tokio::net::UdpSocket;

const HEADER: usize = 12;
const PLAIN_TIMEOUT: Duration = Duration::from_secs(2);
pub const MAX_UDP: usize = 1232;

pub fn truncated(query: &[u8]) -> Option<Vec<u8>> {
	let question_end = HEADER + question_length(query.get(HEADER..)?)?;
	let mut reply = query.get(..question_end)?.to_vec();
	reply[2] = 0x80 | (query[2] & 0x01) | 0x02;
	reply[3] = 0x80;
	reply[6..12].fill(0);
	Some(reply)
}

fn question_length(section: &[u8]) -> Option<usize> {
	let mut offset = 0;
	loop {
		let length = usize::from(*section.get(offset)?);
		offset += 1;
		if length == 0 {
			return (offset + 4 <= section.len()).then_some(offset + 4);
		}
		offset += length;
	}
}

pub async fn plain(servers: &[IpAddr], query: &Bytes) -> Option<Bytes> {
	for server in servers {
		let attempt = async {
			let socket = UdpSocket::bind(SocketAddr::new(IpAddr::from([0, 0, 0, 0]), 0)).await.ok()?;
			socket.send_to(query, SocketAddr::new(*server, 53)).await.ok()?;
			let mut reply = vec![0u8; 4096];
			let length = socket.recv(&mut reply).await.ok()?;
			reply.truncate(length);
			(reply.get(..2) == query.get(..2)).then(|| Bytes::from(reply))
		};
		if let Ok(Some(reply)) = tokio::time::timeout(PLAIN_TIMEOUT, attempt).await {
			return Some(reply);
		}
	}
	None
}
