use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::UdpSocket;

const TYPE_A: u16 = 1;
const SERVER_FAILURE: u8 = 2;
const REFUSED: u8 = 5;

pub fn question(id: u16, name: &str) -> Vec<u8> {
	let mut packet = Vec::with_capacity(name.len() + 18);
	packet.extend_from_slice(&id.to_be_bytes());
	packet.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
	for label in name.split('.') {
		packet.push(label.len() as u8);
		packet.extend_from_slice(label.as_bytes());
	}
	packet.push(0);
	packet.extend_from_slice(&TYPE_A.to_be_bytes());
	packet.extend_from_slice(&1u16.to_be_bytes());
	packet
}

pub async fn answers(server: SocketAddr, name: &str, timeout: Duration) -> bool {
	let attempt = async {
		let socket = UdpSocket::bind("127.0.0.1:0").await.ok()?;
		let mut id = [0u8; 2];
		getrandom::fill(&mut id).ok()?;
		socket.send_to(&question(u16::from_be_bytes(id), name), server).await.ok()?;
		let mut reply = [0u8; 512];
		let length = socket.recv(&mut reply).await.ok()?;
		let rcode = reply.get(3)? & 0x0f;
		(length >= 12 && reply[..2] == id && rcode != SERVER_FAILURE && rcode != REFUSED).then_some(())
	};
	tokio::time::timeout(timeout, attempt).await.ok().flatten().is_some()
}
