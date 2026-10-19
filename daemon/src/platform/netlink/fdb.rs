use std::io;

use super::{NLM_F_DUMP, NLM_F_REQUEST, Socket, find};
use crate::platform::netlink::neighbors::device_name;

const NETLINK_ROUTE: i32 = 0;
const RTM_GETNEIGH: u16 = 30;
const NDMSG_SIZE: usize = 12;
const NDA_LLADDR: u16 = 2;
const NUD_PERMANENT: u16 = 0x80;

pub struct Learned {
	pub mac: String,
	pub port: String,
}

fn parse(reply: &[u8]) -> Option<Learned> {
	let index = u32::from_ne_bytes(reply.get(4..8)?.try_into().ok()?);
	let state = u16::from_ne_bytes(reply.get(8..10)?.try_into().ok()?);
	if state & NUD_PERMANENT != 0 {
		return None;
	}
	let mac = find(reply.get(NDMSG_SIZE..)?, NDA_LLADDR).filter(|raw| raw.len() == 6)?;
	if mac[0] & 1 == 1 {
		return None;
	}
	Some(Learned {
		mac: mac.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":"),
		port: device_name(index)?,
	})
}

pub fn learned() -> io::Result<Vec<Learned>> {
	let mut socket = Socket::open(NETLINK_ROUTE)?;
	let request = [libc::AF_BRIDGE as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
	let replies = socket.request(RTM_GETNEIGH, NLM_F_REQUEST | NLM_F_DUMP, &request)?;
	let mut learned: Vec<Learned> = replies.iter().filter_map(|reply| parse(reply)).collect();
	learned.sort_by(|a, b| (&a.port, &a.mac).cmp(&(&b.port, &b.mac)));
	learned.dedup_by(|a, b| a.port == b.port && a.mac == b.mac);
	Ok(learned)
}
