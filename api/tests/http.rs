//! HTTP-level tests for the api crate, exercised via axum's `tower::ServiceExt::oneshot`
//! against the composed router (no network).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use tasklist_api::{build_router, seed_demo_state};

fn app() -> axum::Router {
    // The SPA dir need not exist for /api tests; those requests never reach the fallback.
    build_router(seed_demo_state(), "web/dist")
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_returns_ok_status() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");
    assert!(json["time"].as_str().is_some_and(|t| !t.is_empty()));
}

#[tokio::test]
async fn tasks_returns_seeded_task_through_the_port() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let tasks = json.as_array().expect("array of tasks");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Welcome to task-list.rs");
    assert_eq!(tasks[0]["owner"], "demo|user");
    assert_eq!(tasks[0]["status"], "todo");
}

#[tokio::test]
async fn openapi_json_is_served() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(
        json["openapi"].as_str().unwrap().split('.').next(),
        Some("3")
    );
    assert!(json["paths"]["/api/health"].is_object());
}

#[tokio::test]
async fn unknown_api_route_is_rfc7807_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert_eq!(content_type, "application/problem+json");
    let json = body_json(response).await;
    assert_eq!(json["status"], 404);
    assert_eq!(json["title"], "Not Found");
}
