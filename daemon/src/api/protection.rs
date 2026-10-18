use axum::Json;
use axum::extract::State;

use crate::config::uci;
use crate::http::{ApiError, ApiResult};
use crate::protection::{self, Settings, View, private_dns};
use crate::state::AppState;

pub async fn overview(State(state): State<AppState>) -> ApiResult<View> {
	Ok(Json(protection::view(&state.ubus).await?))
}

fn valid_domain(domain: &str) -> bool {
	domain.contains('.')
		&& domain
			.split('.')
			.all(|label| !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

pub async fn update(State(state): State<AppState>, Json(mut input): Json<Settings>) -> Result<(), ApiError> {
	let managed: Vec<String> = uci::sections_of(&state.ubus, "cowrt", "network")
		.await?
		.into_iter()
		.map(|s| s.name)
		.collect();
	if let Some(unknown) = input.ads.iter().chain(&input.threats).find(|n| !managed.contains(n)) {
		return Err(ApiError::not_found(format!("There's no network called {unknown}")));
	}
	input.allow = input
		.allow
		.iter()
		.map(|d| d.trim().trim_matches('.').to_ascii_lowercase())
		.filter(|d| !d.is_empty())
		.collect();
	if let Some(bad) = input.allow.iter().find(|d| !valid_domain(d)) {
		return Err(ApiError::bad_request(format!("{bad} isn't a domain")));
	}
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			Ok(protection::store(ubus, &input).await?)
		})
		.await?;
	protection::apply(&state.ubus)
		.await
		.map_err(|err| ApiError::conflict(format!("Blocking couldn't start: {err}")))
}

pub async fn update_dns(State(state): State<AppState>, Json(input): Json<private_dns::Settings>) -> Result<(), ApiError> {
	state
		.applier
		.apply(&state.ubus, &["cowrt"], None, async |ubus| {
			private_dns::store(ubus, &input).await
		})
		.await?;
	Ok(())
}
