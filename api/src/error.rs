use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use tasklist_application::ApplicationError;

/// An API error rendered as an RFC7807 `application/problem+json` response. This is the
/// mapping seam every handler's `Result` funnels through.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    title: &'static str,
    detail: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, title: &'static str, detail: Option<String>) -> Self {
        Self {
            status,
            title,
            detail,
        }
    }
}

/// RFC7807 problem document.
#[derive(Debug, Serialize)]
struct ProblemDetails {
    #[serde(rename = "type")]
    type_uri: &'static str,
    title: &'static str,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ProblemDetails {
            type_uri: "about:blank",
            title: self.title,
            status: self.status.as_u16(),
            detail: self.detail,
        };
        let mut response = (self.status, Json(body)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

impl From<ApplicationError> for ApiError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::Domain(domain) => ApiError::new(
                StatusCode::BAD_REQUEST,
                "Bad Request",
                Some(domain.to_string()),
            ),
            ApplicationError::NotFound => ApiError::new(StatusCode::NOT_FOUND, "Not Found", None),
            // Never leak internal repository details to the client.
            ApplicationError::Repository(_) => ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                None,
            ),
        }
    }
}
