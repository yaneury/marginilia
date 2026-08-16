use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
};
use serde::Deserialize;

use crate::AppState;
use crate::quotes::QuotesResponse;

#[derive(Deserialize)]
pub struct QuotesParams {
    since: Option<u64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/quotes", get(get_quotes))
        .with_state(state)
}

async fn get_quotes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<QuotesParams>,
) -> Result<Json<QuotesResponse>, StatusCode> {
    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if key != state.api_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let since = params.since.unwrap_or(0);
    Ok(Json(state.quotes.as_response(since)))
}
