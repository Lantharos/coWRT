use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::http::{ApiError, ApiResult};
use crate::state::AppState;
use crate::system::{release, updates};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
	current: &'static str,
	latest: String,
	notes: Option<String>,
	available: bool,
	automatic: bool,
}

pub async fn check_update(State(state): State<AppState>) -> ApiResult<UpdateInfo> {
	let release = release::latest().await?;
	Ok(Json(UpdateInfo {
		available: release.is_newer(),
		current: release::CURRENT,
		latest: release.version().to_owned(),
		notes: release.body,
		automatic: updates::automatic(&state.ubus).await?,
	}))
}

pub async fn install_update(State(state): State<AppState>) -> Result<(), ApiError> {
	release::install(&release::latest().await?).await?;
	state.updates.forget_cowrt();
	Ok(())
}

#[derive(Deserialize)]
pub struct AutomaticInput {
	enabled: bool,
}

pub async fn update_automatic(State(state): State<AppState>, Json(input): Json<AutomaticInput>) -> Result<(), ApiError> {
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(updates::set_automatic(ubus, input.enabled).await?)
		})
		.await?;
	Ok(())
}
