use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{NLM_F_ACK, NLM_F_DUMP, NLM_F_REQUEST, Socket, attrs, find};

const NETLINK_NETFILTER: i32 = 12;
const CTNETLINK: u16 = 1 << 8;
const MSG_GET: u16 = CTNETLINK | 1;
const MSG_DELETE: u16 = CTNETLINK | 2;

const TUPLE_ORIG: u16 = 1;
const TUPLE_IP: u16 = 1;
const IP_V4_SRC: u16 = 1;
const IP_V4_DST: u16 = 2;
const IP_V6_SRC: u16 = 3;
const IP_V6_DST: u16 = 4;
const COUNTERS_ORIG: u16 = 9;
const COUNTERS_REPLY: u16 = 10;
const COUNTERS_BYTES: u16 = 2;
const ID: u16 = 12;

pub struct Flow {
	pub id: u32,
	pub family: u8,
	pub source: IpAddr,
	pub destination: IpAddr,
	pub sent: u64,
	pub received: u64,
	tuple: Vec<u8>,
}

fn header(family: u8) -> [u8; 4] {
	[family, 0, 0, 0]
}

fn address(tuple: &[u8], v4: u16, v6: u16) -> Option<IpAddr> {
	let ip = find(tuple, TUPLE_IP)?;
	if let Some(raw) = find(ip, v4) {
		return Some(IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(raw).ok()?)));
	}
	find(ip, v6).and_then(|raw| Some(IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(raw).ok()?))))
}

fn bytes(message: &[u8], kind: u16) -> u64 {
	find(message, kind)
		.and_then(|counters| find(counters, COUNTERS_BYTES))
		.and_then(|raw| raw.try_into().ok())
		.map(u64::from_be_bytes)
		.unwrap_or_default()
}

fn parse(reply: &[u8]) -> Option<Flow> {
	let family = *reply.first()?;
	let message = reply.get(4..)?;
	let tuple = attrs(message).find(|attr| attr.kind == TUPLE_ORIG)?;
	Some(Flow {
		id: find(message, ID)
			.and_then(|raw| raw.try_into().ok())
			.map(u32::from_be_bytes)
			.unwrap_or_default(),
		family,
		source: address(tuple.data, IP_V4_SRC, IP_V6_SRC)?,
		destination: address(tuple.data, IP_V4_DST, IP_V6_DST)?,
		sent: bytes(message, COUNTERS_ORIG),
		received: bytes(message, COUNTERS_REPLY),
		tuple: tuple.raw.to_vec(),
	})
}

pub fn flows() -> io::Result<Vec<Flow>> {
	let mut socket = Socket::open(NETLINK_NETFILTER)?;
	let mut flows = Vec::new();
	for family in [libc::AF_INET, libc::AF_INET6] {
		let replies = socket.request(MSG_GET, NLM_F_REQUEST | NLM_F_DUMP, &header(family as u8))?;
		flows.extend(replies.iter().filter_map(|reply| parse(reply)));
	}
	Ok(flows)
}

pub fn forget(addresses: &[IpAddr]) -> io::Result<()> {
	forget_where(|flow| addresses.contains(&flow.source) || addresses.contains(&flow.destination))
}

pub fn forget_where(doomed: impl Fn(&Flow) -> bool) -> io::Result<()> {
	let mut socket = Socket::open(NETLINK_NETFILTER)?;
	for flow in flows()?.into_iter().filter(|flow| doomed(flow)) {
		let mut payload = header(flow.family).to_vec();
		payload.extend_from_slice(&flow.tuple);
		match socket.request(MSG_DELETE, NLM_F_REQUEST | NLM_F_ACK, &payload) {
			Ok(_) => {}
			Err(err) if err.raw_os_error() == Some(libc::ENOENT) => {}
			Err(err) => return Err(err),
		}
	}
	Ok(())
}
