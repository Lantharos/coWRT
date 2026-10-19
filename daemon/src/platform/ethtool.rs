use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const SIOCETHTOOL: libc::c_ulong = 0x8946;
const ETHTOOL_GSSET_INFO: u32 = 0x37;
const ETHTOOL_GSTRINGS: u32 = 0x1b;
const ETHTOOL_GSTATS: u32 = 0x1d;
const ETH_SS_STATS: u32 = 1;
const STRING_LEN: usize = 32;

#[repr(C)]
struct IfReq {
	name: [libc::c_char; libc::IF_NAMESIZE],
	data: *mut libc::c_void,
	padding: [u8; 16],
}

fn request(socket: &OwnedFd, device: &str, data: *mut libc::c_void) -> io::Result<()> {
	let mut ifreq = IfReq {
		name: [0; libc::IF_NAMESIZE],
		data,
		padding: [0; 16],
	};
	for (slot, byte) in ifreq.name.iter_mut().zip(device.bytes().take(libc::IF_NAMESIZE - 1)) {
		*slot = byte as libc::c_char;
	}
	if unsafe { libc::ioctl(socket.as_raw_fd(), SIOCETHTOOL as _, &mut ifreq) } < 0 {
		return Err(io::Error::last_os_error());
	}
	Ok(())
}

fn count(socket: &OwnedFd, device: &str) -> io::Result<usize> {
	let mut info = [0u64; 3];
	info[0] = u64::from(ETHTOOL_GSSET_INFO);
	info[1] = 1 << ETH_SS_STATS;
	request(socket, device, info.as_mut_ptr().cast())?;
	Ok((info[2] & 0xffff_ffff) as usize)
}

fn names(socket: &OwnedFd, device: &str, count: usize) -> io::Result<Vec<String>> {
	let mut buffer = vec![0u32; 3 + count * STRING_LEN / 4];
	buffer[0] = ETHTOOL_GSTRINGS;
	buffer[1] = ETH_SS_STATS;
	buffer[2] = count as u32;
	request(socket, device, buffer.as_mut_ptr().cast())?;
	let bytes: Vec<u8> = buffer[3..].iter().flat_map(|word| word.to_ne_bytes()).collect();
	Ok(bytes
		.chunks(STRING_LEN)
		.map(|chunk| {
			let end = chunk.iter().position(|b| *b == 0).unwrap_or(chunk.len());
			String::from_utf8_lossy(&chunk[..end]).into_owned()
		})
		.collect())
}

fn values(socket: &OwnedFd, device: &str, count: usize) -> io::Result<Vec<u64>> {
	let mut buffer = vec![0u64; 1 + count];
	buffer[0] = u64::from(ETHTOOL_GSTATS) | (count as u64) << 32;
	request(socket, device, buffer.as_mut_ptr().cast())?;
	Ok(buffer[1..].to_vec())
}

pub fn stats(device: &str, wanted: &[&str]) -> io::Result<Vec<Option<u64>>> {
	let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
	if raw < 0 {
		return Err(io::Error::last_os_error());
	}
	let socket = unsafe { OwnedFd::from_raw_fd(raw) };
	let count = count(&socket, device)?;
	let names = names(&socket, device, count)?;
	let values = values(&socket, device, count)?;
	Ok(wanted
		.iter()
		.map(|name| names.iter().position(|n| n == name).and_then(|i| values.get(i).copied()))
		.collect())
}
