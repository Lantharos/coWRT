pub mod conntrack;
pub mod fdb;
pub mod neighbors;

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const HEADER: usize = 16;
const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
pub const NLM_F_REQUEST: u16 = 0x1;
pub const NLM_F_ACK: u16 = 0x4;
pub const NLM_F_DUMP: u16 = 0x300;
const NLA_TYPE_MASK: u16 = 0x3fff;

const fn align(len: usize) -> usize {
	(len + 3) & !3
}

pub struct Socket {
	fd: OwnedFd,
	seq: u32,
}

impl Socket {
	pub fn open(protocol: i32) -> io::Result<Self> {
		let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_RAW | libc::SOCK_CLOEXEC, protocol) };
		if fd < 0 {
			return Err(io::Error::last_os_error());
		}
		let fd = unsafe { OwnedFd::from_raw_fd(fd) };
		let mut address: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
		address.nl_family = libc::AF_NETLINK as u16;
		let bound = unsafe {
			libc::bind(
				fd.as_raw_fd(),
				(&raw const address).cast(),
				std::mem::size_of::<libc::sockaddr_nl>() as u32,
			)
		};
		if bound < 0 {
			return Err(io::Error::last_os_error());
		}
		Ok(Self { fd, seq: 0 })
	}

	fn send(&mut self, kind: u16, flags: u16, payload: &[u8]) -> io::Result<u32> {
		self.seq += 1;
		let mut message = Vec::with_capacity(HEADER + payload.len());
		message.extend_from_slice(&((HEADER + payload.len()) as u32).to_ne_bytes());
		message.extend_from_slice(&kind.to_ne_bytes());
		message.extend_from_slice(&flags.to_ne_bytes());
		message.extend_from_slice(&self.seq.to_ne_bytes());
		message.extend_from_slice(&0u32.to_ne_bytes());
		message.extend_from_slice(payload);
		let sent = unsafe { libc::send(self.fd.as_raw_fd(), message.as_ptr().cast(), message.len(), 0) };
		if sent < 0 {
			return Err(io::Error::last_os_error());
		}
		Ok(self.seq)
	}

	pub fn request(&mut self, kind: u16, flags: u16, payload: &[u8]) -> io::Result<Vec<Vec<u8>>> {
		let seq = self.send(kind, flags, payload)?;
		let mut buffer = vec![0u8; 64 * 1024];
		let mut replies = Vec::new();
		loop {
			let received = unsafe { libc::recv(self.fd.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len(), 0) };
			if received < 0 {
				return Err(io::Error::last_os_error());
			}
			let mut chunk = &buffer[..received as usize];
			while chunk.len() >= HEADER {
				let len = u32::from_ne_bytes(chunk[0..4].try_into().unwrap()) as usize;
				let kind = u16::from_ne_bytes(chunk[4..6].try_into().unwrap());
				let message_seq = u32::from_ne_bytes(chunk[8..12].try_into().unwrap());
				if len < HEADER || len > chunk.len() {
					break;
				}
				let body = &chunk[HEADER..len];
				chunk = &chunk[align(len).min(chunk.len())..];
				if message_seq != seq {
					continue;
				}
				match kind {
					NLMSG_DONE => return Ok(replies),
					NLMSG_ERROR => {
						let code = i32::from_ne_bytes(body.get(..4).unwrap_or(&[0; 4]).try_into().unwrap());
						return if code == 0 {
							Ok(replies)
						} else {
							Err(io::Error::from_raw_os_error(-code))
						};
					}
					_ => replies.push(body.to_vec()),
				}
			}
			if flags & NLM_F_DUMP == 0 && flags & NLM_F_ACK == 0 {
				return Ok(replies);
			}
		}
	}
}

pub struct Attr<'a> {
	pub kind: u16,
	pub data: &'a [u8],
	pub raw: &'a [u8],
}

pub fn attrs(mut buf: &[u8]) -> impl Iterator<Item = Attr<'_>> {
	std::iter::from_fn(move || {
		let len = u16::from_ne_bytes(buf.get(..2)?.try_into().ok()?) as usize;
		let kind = u16::from_ne_bytes(buf.get(2..4)?.try_into().ok()?) & NLA_TYPE_MASK;
		if len < 4 || len > buf.len() {
			return None;
		}
		let attr = Attr {
			kind,
			data: &buf[4..len],
			raw: &buf[..len],
		};
		buf = &buf[align(len).min(buf.len())..];
		Some(attr)
	})
}

pub fn find(buf: &[u8], kind: u16) -> Option<&[u8]> {
	attrs(buf).find(|attr| attr.kind == kind).map(|attr| attr.data)
}
