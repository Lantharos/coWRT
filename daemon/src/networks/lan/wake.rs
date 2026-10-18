use std::net::{Ipv4Addr, UdpSocket};

use crate::http::ApiError;
use crate::networks::Layout;
use crate::platform::ubus::Ubus;

const PORT: u16 = 9;
const REPEATS: usize = 3;

fn magic_packet(mac: &str) -> Option<Vec<u8>> {
	let bytes: Vec<u8> = mac
		.split(':')
		.map(|octet| u8::from_str_radix(octet, 16))
		.collect::<Result<_, _>>()
		.ok()?;
	(bytes.len() == 6).then(|| {
		[0xff; 6]
			.into_iter()
			.chain(bytes.iter().copied().cycle().take(6 * 16))
			.collect()
	})
}

pub async fn wake(ubus: &Ubus, mac: &str, ip: Ipv4Addr) -> Result<(), ApiError> {
	let broadcast = Layout::load(ubus)
		.await?
		.subnets()
		.into_iter()
		.find(|(_, subnet)| subnet.contains(ip))
		.map(|(_, subnet)| subnet.broadcast())
		.ok_or_else(|| ApiError::conflict("This device isn't on one of your networks"))?;
	let packet = magic_packet(mac).ok_or_else(|| ApiError::bad_request("That doesn't look like a MAC address"))?;
	tokio::task::spawn_blocking(move || {
		let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
		socket.set_broadcast(true)?;
		for _ in 0..REPEATS {
			socket.send_to(&packet, (broadcast, PORT))?;
		}
		Ok::<_, std::io::Error>(())
	})
	.await
	.map_err(|err| ApiError::conflict(err.to_string()))?
	.map_err(|err| ApiError::conflict(format!("Couldn't send the wake-up packet: {err}")))
}
