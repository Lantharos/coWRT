use crate::devices::policy;
use crate::internet::qos;
use crate::networks::Layout;
use crate::state::AppState;

#[derive(Default)]
pub struct Watcher {
	signature: String,
}

async fn ifindex(device: &str) -> String {
	tokio::fs::read_to_string(format!("/sys/class/net/{device}/ifindex"))
		.await
		.unwrap_or_default()
		.trim()
		.to_owned()
}

impl Watcher {
	pub async fn check(&mut self, state: &AppState, wan: Option<&str>) {
		let Ok(layout) = Layout::load(&state.ubus).await else { return };
		let mut signature = String::new();
		for device in policy::lan_devices(&layout).iter().chain(wan.map(str::to_owned).iter()) {
			signature.push_str(&format!("{device}:{} ", ifindex(device).await));
		}
		if signature == self.signature {
			return;
		}
		self.signature = signature;
		reapply(state, wan).await;
	}
}

pub async fn reapply(state: &AppState, wan: Option<&str>) {
	if let Err(err) = policy::apply(&state.ubus).await {
		eprintln!("cowrtd: could not apply device rules: {err}");
	}
	if let Err(err) = qos::apply(&state.ubus, wan).await {
		eprintln!("cowrtd: could not set up the smart queue: {err}");
	}
}
