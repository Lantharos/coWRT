use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use tokio::sync::watch;

use crate::monitor::Snapshot;
use crate::state::AppState;

pub async fn socket(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
	upgrade.on_upgrade(move |socket| stream(state, socket))
}

async fn push(socket: &mut WebSocket, updates: &mut watch::Receiver<Snapshot>) -> bool {
	let Ok(payload) = serde_json::to_string(&*updates.borrow_and_update()) else {
		return false;
	};
	socket.send(Message::Text(payload.into())).await.is_ok()
}

async fn stream(state: AppState, mut socket: WebSocket) {
	let mut updates = state.monitor.subscribe();
	if !push(&mut socket, &mut updates).await {
		return;
	}
	loop {
		tokio::select! {
			changed = updates.changed() => {
				if changed.is_err() || !push(&mut socket, &mut updates).await {
					return;
				}
			}
			incoming = socket.recv() => {
				if matches!(incoming, Some(Ok(Message::Close(_)) | Err(_)) | None) {
					return;
				}
			}
		}
	}
}
