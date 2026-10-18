use axum::Json;
use axum::extract::{Path, State};

use crate::devices::people::{self, Person, PersonInput};
use crate::devices::policy;
use crate::devices::section_id;
use crate::http::{ApiError, ApiResult};
use crate::platform::config::uci;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> ApiResult<Vec<Person>> {
	Ok(Json(people::list(&state.ubus).await?))
}

async fn save(state: &AppState, id: &str, input: &PersonInput) -> Result<(), ApiError> {
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			people::store(ubus, id, input).await
		})
		.await?;
	policy::apply(&state.ubus)
		.await
		.map_err(|err| ApiError::conflict(format!("The router rejected the new rules: {err}")))?;
	if input.blocked {
		for device in uci::sections_of(&state.ubus, "cowrt", "device").await? {
			if device.get("owner") == Some(id)
				&& let Some(mac) = device.get("mac")
			{
				policy::cut_off(mac).await;
			}
		}
	}
	Ok(())
}

pub async fn create(State(state): State<AppState>, Json(input): Json<PersonInput>) -> ApiResult<String> {
	let id = people::new_id();
	save(&state, &id, &input).await?;
	Ok(Json(id))
}

pub async fn update(
	State(state): State<AppState>,
	Path(id): Path<String>,
	Json(input): Json<PersonInput>,
) -> Result<(), ApiError> {
	if !people::list(&state.ubus).await?.iter().any(|p| p.id == id) {
		return Err(ApiError::not_found("That person is gone"));
	}
	save(&state, &id, &input).await
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Result<(), ApiError> {
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(people::remove(ubus, &id).await?)
		})
		.await?;
	policy::apply(&state.ubus)
		.await
		.map_err(|err| ApiError::conflict(format!("The router rejected the new rules: {err}")))?;
	Ok(())
}

pub async fn assign(state: &AppState, mac: &str, owner: Option<&str>) -> Result<(), ApiError> {
	if let Some(owner) = owner
		&& !people::list(&state.ubus).await?.iter().any(|p| p.id == owner)
	{
		return Err(ApiError::bad_request("That person doesn't exist"));
	}
	let section = format!("d{}", section_id(mac));
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			uci::upsert(ubus, "cowrt", "device", &section, serde_json::json!({ "mac": mac })).await?;
			match owner {
				Some(owner) => uci::set(ubus, "cowrt", &section, serde_json::json!({ "owner": owner })).await?,
				None => uci::unset(ubus, "cowrt", &section, &["owner"]).await?,
			}
			Ok(())
		})
		.await?;
	policy::apply(&state.ubus)
		.await
		.map_err(|err| ApiError::conflict(format!("The router rejected the new rules: {err}")))
}
