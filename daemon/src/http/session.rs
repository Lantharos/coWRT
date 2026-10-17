use std::collections::HashMap;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::state::{RUNTIME_DIR, unix_now};

const LIFETIME: Duration = Duration::from_secs(7 * 24 * 3600);
const STORE: &str = "sessions";
pub const COOKIE: &str = "cowrt_session";

pub struct Sessions {
	tokens: Mutex<HashMap<String, u64>>,
	failures: Mutex<u32>,
}

impl Sessions {
	pub async fn load() -> Self {
		let raw = tokio::fs::read_to_string(format!("{RUNTIME_DIR}/{STORE}"))
			.await
			.unwrap_or_default();
		let now = unix_now();
		let tokens = raw
			.lines()
			.filter_map(|line| {
				let (token, expires) = line.split_once(' ')?;
				let expires: u64 = expires.parse().ok()?;
				(expires > now).then(|| (token.to_owned(), expires))
			})
			.collect();
		Self {
			tokens: Mutex::new(tokens),
			failures: Mutex::default(),
		}
	}

	async fn persist(tokens: &HashMap<String, u64>) {
		let contents: String = tokens.iter().map(|(token, expires)| format!("{token} {expires}\n")).collect();
		let file = tokio::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.mode(0o600)
			.open(format!("{RUNTIME_DIR}/{STORE}"))
			.await;
		if let Ok(mut file) = file {
			let _ = file.write_all(contents.as_bytes()).await;
		}
	}

	pub async fn create(&self) -> String {
		let token = crate::secret::token();
		let now = unix_now();
		let mut tokens = self.tokens.lock().await;
		tokens.retain(|_, expires| *expires > now);
		tokens.insert(token.clone(), now + LIFETIME.as_secs());
		Self::persist(&tokens).await;
		*self.failures.lock().await = 0;
		token
	}

	pub async fn validate(&self, token: &str) -> bool {
		let now = unix_now();
		let mut tokens = self.tokens.lock().await;
		match tokens.get_mut(token) {
			Some(expires) if *expires > now => {
				*expires = now + LIFETIME.as_secs();
				true
			}
			_ => false,
		}
	}

	pub async fn revoke(&self, token: &str) {
		let mut tokens = self.tokens.lock().await;
		tokens.remove(token);
		Self::persist(&tokens).await;
	}

	pub async fn penalize(&self) {
		let failures = {
			let mut failures = self.failures.lock().await;
			*failures += 1;
			*failures
		};
		tokio::time::sleep(Duration::from_millis(400) * failures.min(10)).await;
	}

	pub fn cookie(token: &str) -> String {
		format!(
			"{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
			LIFETIME.as_secs()
		)
	}

	pub fn expired_cookie() -> String {
		format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
	}
}
