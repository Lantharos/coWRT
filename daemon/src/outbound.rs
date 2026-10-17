use std::io;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::config::IpFamily;

pub const SERVICE: &str = "https://cowrt.lanth.workers.dev";
const TIMEOUT: Duration = Duration::from_secs(30);

fn agent(family: IpFamily) -> Agent {
	Agent::config_builder()
		.timeout_global(Some(TIMEOUT))
		.ip_family(family)
		.user_agent(concat!("coWRT/", env!("CARGO_PKG_VERSION")))
		.build()
		.into()
}

fn failed(err: ureq::Error) -> io::Error {
	io::Error::other(err.to_string())
}

pub enum Method {
	Get,
	Post,
	Patch,
}

pub struct Request {
	method: Method,
	url: String,
	headers: Vec<(String, String)>,
	ipv4_only: bool,
}

impl Request {
	pub fn new(method: Method, url: impl Into<String>) -> Self {
		Self {
			method,
			url: url.into(),
			headers: Vec::new(),
			ipv4_only: false,
		}
	}

	pub fn get(url: impl Into<String>) -> Self {
		Self::new(Method::Get, url)
	}

	pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
		self.headers.push((name.to_owned(), value.into()));
		self
	}

	pub fn ipv4_only(mut self) -> Self {
		self.ipv4_only = true;
		self
	}

	fn run(self, body: Option<Vec<u8>>) -> io::Result<ureq::http::Response<ureq::Body>> {
		let agent = agent(if self.ipv4_only { IpFamily::Ipv4Only } else { IpFamily::Any });
		macro_rules! send {
			($builder:expr) => {{
				let mut builder = $builder;
				for (name, value) in &self.headers {
					builder = builder.header(name, value);
				}
				builder
			}};
		}
		match (self.method, body) {
			(Method::Get, _) => send!(agent.get(&self.url)).call(),
			(Method::Post, body) => send!(agent.post(&self.url))
				.content_type("application/json")
				.send(body.unwrap_or_default()),
			(Method::Patch, body) => send!(agent.patch(&self.url))
				.content_type("application/json")
				.send(body.unwrap_or_default()),
		}
		.map_err(failed)
	}

	pub async fn json<T: DeserializeOwned + Send + 'static>(self, body: Option<impl Serialize>) -> io::Result<T> {
		let body = body.map(|body| serde_json::to_vec(&body)).transpose()?;
		tokio::task::spawn_blocking(move || self.run(body)?.body_mut().read_json().map_err(failed)).await?
	}

	pub async fn bytes(self, limit: u64) -> io::Result<Vec<u8>> {
		tokio::task::spawn_blocking(move || {
			self.run(None)?
				.body_mut()
				.with_config()
				.limit(limit)
				.read_to_vec()
				.map_err(failed)
		})
		.await?
	}
}
