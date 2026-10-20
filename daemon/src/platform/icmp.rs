use std::io;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use super::socket::option;

const ECHO_REQUEST: u8 = 8;
const ECHO_REPLY: u8 = 0;
const ICMP_FILTER: i32 = 1;
const PAYLOAD: [u8; 8] = *b"cowrt-ln";

pub struct Pinger {
	fd: AsyncFd<OwnedFd>,
	id: u16,
}

fn checksum(packet: &[u8]) -> u16 {
	let mut sum: u32 = packet
		.chunks(2)
		.map(|pair| u32::from(u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)])))
		.sum();
	while sum > 0xffff {
		sum = (sum & 0xffff) + (sum >> 16);
	}
	!(sum as u16)
}

fn address(ip: Ipv4Addr) -> libc::sockaddr_in {
	libc::sockaddr_in {
		sin_family: libc::AF_INET as libc::sa_family_t,
		sin_port: 0,
		sin_addr: libc::in_addr {
			s_addr: u32::from_ne_bytes(ip.octets()),
		},
		sin_zero: [0; 8],
	}
}

impl Pinger {
	pub fn open(device: &str) -> io::Result<Self> {
		let raw = unsafe {
			libc::socket(
				libc::AF_INET,
				libc::SOCK_RAW | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
				libc::IPPROTO_ICMP,
			)
		};
		if raw < 0 {
			return Err(io::Error::last_os_error());
		}
		let fd = unsafe { OwnedFd::from_raw_fd(raw) };
		let name = device.as_bytes();
		let bound = unsafe {
			libc::setsockopt(
				raw,
				libc::SOL_SOCKET,
				libc::SO_BINDTODEVICE,
				name.as_ptr().cast(),
				name.len() as libc::socklen_t,
			)
		};
		if bound < 0 {
			return Err(io::Error::last_os_error());
		}
		option(raw, libc::SOL_RAW, ICMP_FILTER, &!(1u32 << ECHO_REPLY))?;
		let mut id = [0u8; 2];
		getrandom::fill(&mut id).map_err(io::Error::other)?;
		Ok(Self {
			fd: unsafe { AsyncFd::register_with_interest(fd, Interest::READABLE) }?,
			id: u16::from_ne_bytes(id),
		})
	}

	pub fn send(&self, target: Ipv4Addr, sequence: u16) -> io::Result<()> {
		let mut packet = [0u8; 8 + PAYLOAD.len()];
		packet[0] = ECHO_REQUEST;
		packet[4..6].copy_from_slice(&self.id.to_be_bytes());
		packet[6..8].copy_from_slice(&sequence.to_be_bytes());
		packet[8..].copy_from_slice(&PAYLOAD);
		let sum = checksum(&packet);
		packet[2..4].copy_from_slice(&sum.to_be_bytes());
		let to = address(target);
		let sent = unsafe {
			libc::sendto(
				self.fd.as_raw_fd(),
				packet.as_ptr().cast(),
				packet.len(),
				0,
				(&raw const to).cast(),
				size_of::<libc::sockaddr_in>() as libc::socklen_t,
			)
		};
		if sent < 0 {
			return Err(io::Error::last_os_error());
		}
		Ok(())
	}

	fn reply(&self, packet: &[u8]) -> Option<u16> {
		let header = usize::from(packet.first()? & 0x0f) * 4;
		let icmp = packet.get(header..header + 8)?;
		let id = u16::from_be_bytes([icmp[4], icmp[5]]);
		(icmp[0] == ECHO_REPLY && id == self.id).then(|| u16::from_be_bytes([icmp[6], icmp[7]]))
	}

	pub async fn receive(&self) -> io::Result<u16> {
		let mut buffer = [0u8; 128];
		loop {
			let mut ready = self.fd.readable().await?;
			let received = ready.try_io(|fd| {
				let read = unsafe { libc::recv(fd.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len(), 0) };
				if read < 0 {
					return Err(io::Error::last_os_error());
				}
				Ok(read as usize)
			});
			match received {
				Ok(Ok(length)) => {
					if let Some(sequence) = self.reply(&buffer[..length]) {
						return Ok(sequence);
					}
				}
				Ok(Err(err)) => return Err(err),
				Err(_would_block) => {}
			}
		}
	}
}
