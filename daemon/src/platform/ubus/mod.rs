mod blob;
mod client;

pub use client::Ubus;

pub const STATUS_NOT_FOUND: u32 = 4;

#[derive(Debug)]
pub enum Error {
	Io(std::io::Error),
	Json(serde_json::Error),
	Status(u32),
	Disconnected,
	Timeout,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Io(err) => write!(f, "ubus socket: {err}"),
			Self::Json(err) => write!(f, "unexpected ubus reply: {err}"),
			Self::Status(code) => write!(f, "ubus status {code}"),
			Self::Disconnected => f.write_str("ubus disconnected"),
			Self::Timeout => f.write_str("ubus call timed out"),
		}
	}
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
	fn from(err: std::io::Error) -> Self {
		Self::Io(err)
	}
}

impl From<serde_json::Error> for Error {
	fn from(err: serde_json::Error) -> Self {
		Self::Json(err)
	}
}
