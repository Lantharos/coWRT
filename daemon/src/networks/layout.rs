use crate::platform::config::uci::{self, Section};
use crate::platform::ubus::{self, Ubus};

use super::address::Cidr;

pub const PRIMARY: &str = "lan";
pub const DEFAULT_BRIDGE: &str = "br-lan";

pub struct Membership {
	pub port: String,
	pub untagged: bool,
}

impl Membership {
	pub fn parse(entry: &str) -> Self {
		let (port, flags) = entry.split_once(':').unwrap_or((entry, ""));
		Self {
			port: port.to_owned(),
			untagged: flags.contains('u'),
		}
	}

	pub fn untagged(port: &str) -> String {
		format!("{port}:u*")
	}

	pub fn tagged(port: &str) -> String {
		format!("{port}:t")
	}
}

pub struct Layout {
	pub network: Vec<Section>,
	pub dhcp: Vec<Section>,
	pub wireless: Vec<Section>,
	pub cowrt: Vec<Section>,
}

impl Layout {
	pub async fn load(ubus: &Ubus) -> Result<Self, ubus::Error> {
		let (network, dhcp, wireless, cowrt) = tokio::try_join!(
			uci::sections(ubus, "network"),
			uci::sections(ubus, "dhcp"),
			uci::sections(ubus, "wireless"),
			uci::sections(ubus, "cowrt"),
		)?;
		Ok(Self {
			network,
			dhcp,
			wireless,
			cowrt,
		})
	}

	pub fn managed(&self) -> impl Iterator<Item = &Section> {
		self.cowrt.iter().filter(|s| s.kind == "network")
	}

	pub fn interface(&self, id: &str) -> Option<&Section> {
		self.network.iter().find(|s| s.kind == "interface" && s.name == id)
	}

	pub fn bridge_name(&self) -> String {
		self.interface(PRIMARY)
			.and_then(|lan| lan.get("device"))
			.map(|device| device.split('.').next().unwrap_or(device).to_owned())
			.unwrap_or_else(|| DEFAULT_BRIDGE.to_owned())
	}

	pub fn bridge(&self) -> Option<&Section> {
		let name = self.bridge_name();
		self.network
			.iter()
			.find(|s| s.kind == "device" && s.get("name") == Some(&name))
	}

	pub fn bridge_ports(&self) -> Vec<String> {
		self.bridge().map(|bridge| bridge.list("ports")).unwrap_or_default()
	}

	pub fn vlan_of(&self, id: &str) -> Option<u16> {
		let device = self.interface(id)?.get("device")?;
		device.rsplit_once('.')?.1.parse().ok()
	}

	pub fn vlan_section(&self, vlan: u16) -> Option<&Section> {
		let bridge = self.bridge_name();
		self.network.iter().find(|s| {
			s.kind == "bridge-vlan"
				&& s.get("device") == Some(&bridge)
				&& s.get("vlan").and_then(|v| v.parse().ok()) == Some(vlan)
		})
	}

	pub fn members(&self, vlan: u16) -> Vec<Membership> {
		self.vlan_section(vlan)
			.map(|s| s.list("ports").iter().map(|p| Membership::parse(p)).collect())
			.unwrap_or_default()
	}

	pub fn address(&self, id: &str) -> Option<Cidr> {
		Cidr::from_interface(self.interface(id)?)
	}

	pub fn dhcp(&self, id: &str) -> Option<&Section> {
		self.dhcp.iter().find(|s| s.kind == "dhcp" && s.get("interface") == Some(id))
	}

	pub fn wan_devices(&self) -> Vec<String> {
		["wan", "wan6"]
			.iter()
			.filter_map(|id| self.interface(id)?.get("device").map(str::to_owned))
			.collect()
	}

	pub fn subnets(&self) -> Vec<(String, Cidr)> {
		self.managed()
			.filter_map(|n| Some((n.name.clone(), self.address(&n.name)?)))
			.collect()
	}
}
