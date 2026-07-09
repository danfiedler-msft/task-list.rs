use axum::extract::State;
use axum::Json;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;

use crate::state::AppState;

/// Health payload. `status` is always `ok`; `time` is sourced through the `Clock` port,
/// proving that seam is wired end to end.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Always `"ok"` when the service is live.
    pub status: String,
    /// Server time (RFC3339).
    pub time: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses((status = 200, description = "Service is live", body = HealthResponse))
)]
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let time = state.clock.now().format(&Rfc3339).unwrap_or_default();
    Json(HealthResponse {
        status: "ok".to_owned(),
        time,
    })
}
