mod api;
mod config;
mod ddns;
mod devices;
mod forwards;
mod http;
mod internet;
mod monitor;
mod netlink;
mod networks;
mod outbound;
mod packages;
mod policy;
mod qos;
mod secret;
mod state;
mod ubus;
mod vpn;
mod wifi;

use std::net::SocketAddr;
use std::path::PathBuf;

use state::AppState;

struct Args {
	listen: SocketAddr,
	www: PathBuf,
}

impl Args {
	fn parse() -> Self {
		let mut args = Self {
			listen: SocketAddr::from(([0, 0, 0, 0], 80)),
			www: PathBuf::from("/usr/share/cowrt/www"),
		};
		let mut iter = std::env::args().skip(1);
		while let Some(flag) = iter.next() {
			let value = iter.next().unwrap_or_else(|| panic!("{flag} needs a value"));
			match flag.as_str() {
				"--listen" => args.listen = value.parse().expect("--listen expects an address like 0.0.0.0:80"),
				"--www" => args.www = value.into(),
				_ => panic!("unknown flag {flag}"),
			}
		}
		args
	}
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
	let args = Args::parse();
	let state = AppState::load().await;
	if let Err(err) = networks::adopt(&state.ubus).await {
		eprintln!("cowrtd: could not set up the network layout: {err}");
	}
	monitor::spawn(state.clone());
	ddns::spawn(state.clone());

	let listener = tokio::net::TcpListener::bind(args.listen)
		.await
		.expect("failed to bind listen address");
	eprintln!("cowrtd listening on {}", args.listen);
	axum::serve(
		listener,
		http::router(state.clone(), &args.www).into_make_service_with_connect_info::<SocketAddr>(),
	)
	.with_graceful_shutdown(shutdown())
	.await
	.expect("server error");
	state.monitor.save().await;
}

async fn shutdown() {
	let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler");
	tokio::select! {
		_ = terminate.recv() => {}
		_ = tokio::signal::ctrl_c() => {}
	}
}
