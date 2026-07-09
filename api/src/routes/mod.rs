pub mod health;
pub mod tasks;

use axum::http::StatusCode;
use axum::Json;
use utoipa::OpenApi;

use crate::error::ApiError;
use crate::openapi::ApiDoc;

/// Serve the OpenAPI document at `/api/openapi.json`.
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Fallback for unmatched `/api/*` routes — an RFC7807 404 (never the SPA shell).
pub async fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "Not Found",
        Some("no such API route".to_owned()),
    )
}
