use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Security {
	Wpa3,
	Wpa2Wpa3,
	Wpa2,
	EnhancedOpen,
	Open,
}

impl Security {
	pub fn from_uci(encryption: &str) -> Self {
		match encryption.split('+').next().unwrap_or_default() {
			"sae" => Self::Wpa3,
			"sae-mixed" => Self::Wpa2Wpa3,
			"psk2" | "psk-mixed" | "psk" => Self::Wpa2,
			"owe" => Self::EnhancedOpen,
			_ => Self::Open,
		}
	}

	pub fn uci(self) -> &'static str {
		match self {
			Self::Wpa3 => "sae",
			Self::Wpa2Wpa3 => "sae-mixed",
			Self::Wpa2 => "psk2",
			Self::EnhancedOpen => "owe",
			Self::Open => "none",
		}
	}

	pub fn needs_password(self) -> bool {
		matches!(self, Self::Wpa3 | Self::Wpa2Wpa3 | Self::Wpa2)
	}
}
