use std::ffi::CStr;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{NLM_F_DUMP, NLM_F_REQUEST, Socket, find};

const NETLINK_ROUTE: i32 = 0;
const RTM_GETNEIGH: u16 = 30;
const NDMSG_SIZE: usize = 12;
const NDA_DST: u16 = 1;
const NDA_LLADDR: u16 = 2;

const NUD_REACHABLE: u16 = 0x02;
const NUD_STALE: u16 = 0x04;
const NUD_DELAY: u16 = 0x08;
const NUD_PROBE: u16 = 0x10;

pub struct Neighbor {
	pub ip: IpAddr,
	pub mac: String,
	pub device: String,
	pub active: bool,
}

pub fn device_name(index: u32) -> Option<String> {
	let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
	let found = unsafe { libc::if_indextoname(index, name.as_mut_ptr()) };
	(!found.is_null()).then(|| unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy().into_owned())
}

fn parse(reply: &[u8]) -> Option<Neighbor> {
	let index = u32::from_ne_bytes(reply.get(4..8)?.try_into().ok()?);
	let state = u16::from_ne_bytes(reply.get(8..10)?.try_into().ok()?);
	if state & (NUD_REACHABLE | NUD_STALE | NUD_DELAY | NUD_PROBE) == 0 {
		return None;
	}
	let attrs = reply.get(NDMSG_SIZE..)?;
	let ip = match find(attrs, NDA_DST)? {
		raw if raw.len() == 4 => IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(raw).ok()?)),
		raw if raw.len() == 16 => IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(raw).ok()?)),
		_ => return None,
	};
	let mac = find(attrs, NDA_LLADDR).filter(|raw| raw.len() == 6)?;
	Some(Neighbor {
		ip,
		mac: mac.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":"),
		device: device_name(index)?,
		active: state & (NUD_REACHABLE | NUD_DELAY | NUD_PROBE) != 0,
	})
}

pub fn list() -> io::Result<Vec<Neighbor>> {
	let mut socket = Socket::open(NETLINK_ROUTE)?;
	let request = [libc::AF_UNSPEC as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
	let replies = socket.request(RTM_GETNEIGH, NLM_F_REQUEST | NLM_F_DUMP, &request)?;
	Ok(replies.iter().filter_map(|reply| parse(reply)).collect())
}
