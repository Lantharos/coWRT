use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

use crate::config::uci::Section;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
	pub addr: Ipv4Addr,
	pub prefix: u8,
}

impl Cidr {
	pub fn new(addr: Ipv4Addr, prefix: u8) -> Self {
		Self { addr, prefix }
	}

	pub fn with_host(self, addr: Ipv4Addr) -> Self {
		Self {
			addr,
			prefix: self.prefix,
		}
	}

	pub fn mask(self) -> u32 {
		u32::MAX.checked_shl(32 - u32::from(self.prefix)).unwrap_or(0)
	}

	pub fn network(self) -> u32 {
		u32::from(self.addr) & self.mask()
	}

	pub fn contains(self, ip: Ipv4Addr) -> bool {
		u32::from(ip) & self.mask() == self.network()
	}

	pub fn overlaps(self, other: Cidr) -> bool {
		let mask = self.mask() & other.mask();
		u32::from(self.addr) & mask == u32::from(other.addr) & mask
	}

	pub fn is_usable_host(self) -> bool {
		let host = u32::from(self.addr);
		(8..=30).contains(&self.prefix) && host != self.network() && host != self.network() | !self.mask()
	}

	pub fn translate(self, ip: Ipv4Addr, to: Cidr) -> Ipv4Addr {
		let host = u32::from(ip) & !self.mask() & !to.mask();
		Ipv4Addr::from(to.network() | host)
	}

	pub fn from_interface(section: &Section) -> Option<Self> {
		let first = section.list("ipaddr").into_iter().next()?;
		if first.contains('/') {
			return first.parse().ok();
		}
		let addr = first.parse().ok()?;
		let mask: Ipv4Addr = section.get("netmask")?.parse().ok()?;
		Some(Self::new(addr, u32::from(mask).count_ones() as u8))
	}
}

impl FromStr for Cidr {
	type Err = ();

	fn from_str(value: &str) -> Result<Self, ()> {
		let (addr, prefix) = value.split_once('/').ok_or(())?;
		let prefix: u8 = prefix.parse().map_err(drop)?;
		if prefix > 32 {
			return Err(());
		}
		Ok(Self::new(addr.parse().map_err(drop)?, prefix))
	}
}

impl fmt::Display for Cidr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}/{}", self.addr, self.prefix)
	}
}
