use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::{Duration, Instant};

const GROUP: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 0, 251), 5353);
const LISTEN: Duration = Duration::from_millis(1500);
const PTR: u16 = 12;
const CLASS_IN: u16 = 1;
const MAX_QUESTIONS: usize = 32;

fn reverse_name(ip: Ipv4Addr) -> String {
	let [a, b, c, d] = ip.octets();
	format!("{d}.{c}.{b}.{a}.in-addr.arpa")
}

fn encode_name(name: &str, out: &mut Vec<u8>) {
	for label in name.split('.') {
		out.push(label.len() as u8);
		out.extend_from_slice(label.as_bytes());
	}
	out.push(0);
}

fn query(targets: &[Ipv4Addr]) -> Vec<u8> {
	let mut packet = vec![0, 0, 0, 0, 0, targets.len() as u8, 0, 0, 0, 0, 0, 0];
	for ip in targets {
		encode_name(&reverse_name(*ip), &mut packet);
		packet.extend_from_slice(&PTR.to_be_bytes());
		packet.extend_from_slice(&CLASS_IN.to_be_bytes());
	}
	packet
}

fn read_name(packet: &[u8], mut offset: usize) -> Option<(String, usize)> {
	let mut labels = Vec::new();
	let mut end = None;
	for _ in 0..64 {
		let length = *packet.get(offset)? as usize;
		if length == 0 {
			return Some((labels.join("."), end.unwrap_or(offset + 1)));
		}
		if length & 0xc0 == 0xc0 {
			let pointer = (length & 0x3f) << 8 | *packet.get(offset + 1)? as usize;
			end.get_or_insert(offset + 2);
			offset = pointer;
			continue;
		}
		labels.push(String::from_utf8_lossy(packet.get(offset + 1..offset + 1 + length)?).into_owned());
		offset += 1 + length;
	}
	None
}

fn u16_at(packet: &[u8], offset: usize) -> Option<u16> {
	Some(u16::from_be_bytes([*packet.get(offset)?, *packet.get(offset + 1)?]))
}

fn answers(packet: &[u8]) -> Vec<(String, String)> {
	let mut found = Vec::new();
	let (Some(questions), Some(records)) = (u16_at(packet, 4), u16_at(packet, 6)) else {
		return found;
	};
	let mut offset = 12;
	for _ in 0..questions {
		let Some((_, next)) = read_name(packet, offset) else {
			return found;
		};
		offset = next + 4;
	}
	for _ in 0..records {
		let Some((name, next)) = read_name(packet, offset) else {
			return found;
		};
		let (Some(kind), Some(length)) = (u16_at(packet, next), u16_at(packet, next + 8)) else {
			return found;
		};
		let data = next + 10;
		if kind == PTR
			&& let Some((target, _)) = read_name(packet, data)
		{
			found.push((name, target));
		}
		offset = data + length as usize;
	}
	found
}

fn option<T>(fd: i32, level: i32, name: i32, value: &T) -> io::Result<()> {
	let result = unsafe { libc::setsockopt(fd, level, name, (value as *const T).cast(), size_of::<T>() as libc::socklen_t) };
	if result < 0 {
		return Err(io::Error::last_os_error());
	}
	Ok(())
}

fn shared_socket(local: Ipv4Addr) -> io::Result<UdpSocket> {
	let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
	if fd < 0 {
		return Err(io::Error::last_os_error());
	}
	let socket = unsafe { UdpSocket::from_raw_fd(fd) };
	option(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, &1i32)?;
	option(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, &1i32)?;
	let any = libc::sockaddr_in {
		sin_family: libc::AF_INET as libc::sa_family_t,
		sin_port: GROUP.port().to_be(),
		sin_addr: libc::in_addr { s_addr: 0 },
		sin_zero: [0; 8],
	};
	let bound = unsafe { libc::bind(fd, (&raw const any).cast(), size_of::<libc::sockaddr_in>() as libc::socklen_t) };
	if bound < 0 {
		return Err(io::Error::last_os_error());
	}
	socket.join_multicast_v4(GROUP.ip(), &local)?;
	let interface = libc::in_addr {
		s_addr: u32::from_ne_bytes(local.octets()),
	};
	option(socket.as_raw_fd(), libc::IPPROTO_IP, libc::IP_MULTICAST_IF, &interface)?;
	socket.set_multicast_ttl_v4(255)?;
	Ok(socket)
}

pub fn names(local: Ipv4Addr, targets: &[Ipv4Addr]) -> io::Result<HashMap<Ipv4Addr, String>> {
	let socket = shared_socket(local)?;
	for chunk in targets.chunks(MAX_QUESTIONS) {
		socket.send_to(&query(chunk), GROUP)?;
	}
	let wanted: HashMap<String, Ipv4Addr> = targets.iter().map(|ip| (reverse_name(*ip), *ip)).collect();
	let mut found = HashMap::new();
	let mut buffer = [0u8; 9000];
	let deadline = Instant::now() + LISTEN;
	while let Some(left) = deadline.checked_duration_since(Instant::now()) {
		socket.set_read_timeout(Some(left.max(Duration::from_millis(1))))?;
		let Ok((size, _)) = socket.recv_from(&mut buffer) else { break };
		for (name, target) in answers(&buffer[..size]) {
			if let Some(ip) = wanted.get(&name.to_ascii_lowercase()) {
				found.insert(*ip, target);
			}
		}
	}
	Ok(found)
}
