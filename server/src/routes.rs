use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
};

use crate::AppState;
use crate::quotes::QuotesResponse;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/quotes", get(get_quotes))
        .with_state(state)
}

async fn get_quotes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QuotesResponse>, StatusCode> {
    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if key != state.api_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(Json(state.quotes.as_response()))
}
