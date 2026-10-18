use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::http::ApiError;
use crate::platform::ubus::Ubus;
use crate::system::secret;

const CONFIG_DIR: &str = "/etc/config";

struct Snapshot {
	configs: Vec<(String, Vec<u8>)>,
}

impl Snapshot {
	async fn take(configs: &[&str]) -> Self {
		let mut saved = Vec::with_capacity(configs.len());
		for config in configs {
			saved.push((
				config.to_string(),
				tokio::fs::read(format!("{CONFIG_DIR}/{config}")).await.unwrap_or_default(),
			));
		}
		Self { configs: saved }
	}

	async fn restore(&self, ubus: &Ubus) {
		for (config, contents) in &self.configs {
			let _ = ubus.call("uci", "revert", json!({ "config": config })).await;
			let _ = tokio::fs::write(format!("{CONFIG_DIR}/{config}"), contents).await;
		}
		reload(ubus, self.configs.iter().map(|(config, _)| config.as_str())).await;
	}
}

struct Pending {
	token: String,
	rollback: JoinHandle<()>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
	pub confirm: Option<Confirmation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmation {
	pub token: String,
	pub seconds: u64,
}

#[derive(Default)]
pub struct Applier {
	busy: Mutex<()>,
	pending: Arc<Mutex<Option<Pending>>>,
}

pub async fn reload<'a>(ubus: &Ubus, configs: impl Iterator<Item = &'a str>) {
	for config in configs {
		let _ = ubus
			.call(
				"service",
				"event",
				json!({ "type": "config.change", "data": { "package": config } }),
			)
			.await;
	}
}

impl Applier {
	pub async fn apply<F>(&self, ubus: &Ubus, configs: &[&str], rollback: Option<Duration>, edit: F) -> Result<Applied, ApiError>
	where
		F: AsyncFnOnce(&Ubus) -> Result<(), ApiError>,
	{
		let _busy = self.busy.lock().await;
		if self.pending.lock().await.is_some() {
			return Err(ApiError::conflict("Another change is still waiting to be confirmed"));
		}

		let snapshot = Snapshot::take(configs).await;
		if let Err(err) = edit(ubus).await {
			for config in configs {
				let _ = ubus.call("uci", "revert", json!({ "config": config })).await;
			}
			return Err(err);
		}
		for config in configs {
			ubus.call("uci", "commit", json!({ "config": config })).await?;
		}
		reload(ubus, configs.iter().copied()).await;

		let Some(window) = rollback else {
			return Ok(Applied { confirm: None });
		};
		let token = secret::token();
		let expected = token.clone();
		let pending = self.pending.clone();
		let bus = ubus.clone();
		let rollback = tokio::spawn(async move {
			tokio::time::sleep(window).await;
			let mut slot = pending.lock().await;
			if slot.as_ref().is_some_and(|pending| pending.token == expected) {
				*slot = None;
				drop(slot);
				eprintln!("cowrtd: change was not confirmed, rolling back");
				snapshot.restore(&bus).await;
			}
		});
		*self.pending.lock().await = Some(Pending {
			token: token.clone(),
			rollback,
		});
		Ok(Applied {
			confirm: Some(Confirmation {
				token,
				seconds: window.as_secs(),
			}),
		})
	}

	pub async fn confirm(&self, token: &str) -> Result<(), ApiError> {
		let mut slot = self.pending.lock().await;
		match slot.take_if(|pending| pending.token == token) {
			Some(pending) => {
				pending.rollback.abort();
				Ok(())
			}
			None => Err(ApiError::conflict("This change was already rolled back")),
		}
	}
}
