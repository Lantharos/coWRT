use std::io;
use std::sync::LazyLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::monitor::wan::Wan;
use crate::platform::config::uci;
use crate::platform::ubus::Ubus;
use crate::state::AppState;
use crate::system::outbound::{Method, Request, SERVICE};

const PROBE_RULE: &str = "cw_probe";
const WATCH: Duration = Duration::from_secs(600);
const RECHECK: Duration = Duration::from_secs(6 * 3600);

struct Verdict {
	public_ip: Option<String>,
	reachable: bool,
}

static VERDICT: LazyLock<Mutex<Option<Verdict>>> = LazyLock::new(Mutex::default);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reach {
	pub public_ip: Option<String>,
	pub wan_ip: Option<String>,
	pub behind_nat: bool,
	pub reachable: Option<bool>,
}

#[derive(Deserialize)]
struct Ip {
	ip: String,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Probe {
	pub port: u16,
	pub open: bool,
}

#[derive(Deserialize)]
struct Probes {
	results: Vec<Probe>,
}

pub async fn public_ipv4() -> io::Result<String> {
	let reply: Ip = Request::get(format!("{SERVICE}/ip")).ipv4_only().json(None::<()>).await?;
	Ok(reply.ip)
}

pub async fn status(ubus: &Ubus) -> Reach {
	let public_ip = public_ipv4().await.ok();
	let wan_ip = Wan::read(ubus, crate::monitor::wan::MAIN)
		.await
		.ipv4
		.map(|cidr| cidr.split('/').next().unwrap_or_default().to_owned());
	let behind_nat = matches!((&public_ip, &wan_ip), (Some(public), Some(wan)) if public != wan);
	let reachable = VERDICT
		.try_lock()
		.ok()
		.as_deref()
		.and_then(Option::as_ref)
		.filter(|verdict| verdict.public_ip == public_ip)
		.map(|verdict| verdict.reachable);
	Reach {
		public_ip,
		wan_ip,
		behind_nat,
		reachable,
	}
}

pub async fn check(ports: &[u16]) -> io::Result<Vec<Probe>> {
	let reply: Probes = Request::new(Method::Post, format!("{SERVICE}/check"))
		.ipv4_only()
		.json(Some(json!({ "ports": ports })))
		.await?;
	Ok(reply.results)
}

async fn firewall_reload() -> io::Result<()> {
	Command::new("fw4").arg("-q").arg("reload").status().await.map(drop)
}

pub async fn inbound(ubus: &Ubus) -> io::Result<bool> {
	let mut verdict = VERDICT.lock().await;
	let public_ip = public_ipv4().await.ok();
	let reachable = probe_inbound(ubus).await?;
	*verdict = Some(Verdict { public_ip, reachable });
	Ok(reachable)
}

async fn probe_inbound(ubus: &Ubus) -> io::Result<bool> {
	let mut random = [0u8; 2];
	getrandom::fill(&mut random).map_err(io::Error::other)?;
	let port = 40_000 + u16::from_be_bytes(random) % 10_000;
	let listener = TcpListener::bind(("0.0.0.0", port)).await?;
	let rule = json!({ "name": "coWRT reachability test", "src": "wan", "proto": "tcp", "dest_port": port.to_string(), "target": "ACCEPT" });
	uci::add(ubus, "firewall", "rule", Some(PROBE_RULE), rule)
		.await
		.map_err(io::Error::other)?;
	ubus.call("uci", "commit", json!({ "config": "firewall" }))
		.await
		.map_err(io::Error::other)?;
	firewall_reload().await?;

	let result = check(&[port]).await;

	drop(listener);
	let _ = uci::delete(ubus, "firewall", PROBE_RULE).await;
	let _ = ubus.call("uci", "commit", json!({ "config": "firewall" })).await;
	firewall_reload().await?;
	Ok(result?.first().is_some_and(|probe| probe.open))
}

pub fn spawn(state: AppState) {
	tokio::spawn(async move {
		let mut since_check = RECHECK;
		loop {
			let public_ip = public_ipv4().await.ok();
			let known = VERDICT.lock().await.as_ref().map(|verdict| verdict.public_ip.clone());
			if public_ip.is_some() && (since_check >= RECHECK || known != Some(public_ip)) {
				if let Err(err) = inbound(&state.ubus).await {
					eprintln!("cowrtd: could not check whether home is reachable: {err}");
				}
				since_check = Duration::ZERO;
			}
			tokio::time::sleep(WATCH).await;
			since_check += WATCH;
		}
	});
}
