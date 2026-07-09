//! API layer: axum host, routers, OpenAPI, RFC7807 error mapping, SPA serving and the
//! composition root. Depends on `application` + `infrastructure` + `domain` and wires
//! ports to adapters via `AppState`.

mod error;
mod openapi;
mod routes;
mod spa;
mod state;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use tasklist_application::ports::Clock;
use tasklist_domain::{Task, TaskId, TaskStatus, Title, UserId};
use tasklist_infrastructure::{InMemoryTaskRepository, SystemClock};

pub use error::ApiError;
pub use openapi::ApiDoc;
pub use state::AppState;

/// Build the full application router: `/api/*` (health, tasks, openapi.json) with an
/// RFC7807 fallback, the Swagger UI at `/swagger`, and the SPA served with an
/// `index.html` fallback for everything else.
pub fn build_router(state: AppState, web_dist_dir: &str) -> Router {
    let api = Router::new()
        .route("/health", get(routes::health::health))
        .route("/tasks", get(routes::tasks::list_tasks))
        .route("/openapi.json", get(routes::openapi_json))
        .fallback(routes::not_found)
        .with_state(state);

    Router::new()
        .nest("/api", api)
        // Swagger UI renders the spec fetched from our own `/api/openapi.json` handler,
        // so the canonical spec URL is served in exactly one place.
        .merge(SwaggerUi::new("/swagger").config(Config::new(["/api/openapi.json"])))
        .fallback_service(spa::spa_service(web_dist_dir))
        .layer(TraceLayer::new_for_http())
}

/// Compose the walking-skeleton [`AppState`]: an in-memory repository seeded with a demo
/// task and a system clock. The demo owner stands in until Auth lands in a later task.
pub fn seed_demo_state() -> AppState {
    let demo_owner = UserId::parse("demo|user").expect("demo owner is valid");
    let clock = SystemClock;
    let demo_task = Task::new(
        TaskId::new(),
        demo_owner.clone(),
        Title::parse("Welcome to task-list.rs").expect("demo title is valid"),
        TaskStatus::Todo,
        clock.now(),
    );

    AppState {
        repo: Arc::new(InMemoryTaskRepository::seeded(vec![demo_task])),
        clock: Arc::new(clock),
        demo_owner,
    }
}

/// The OpenAPI document as pretty JSON. Emitted by the `openapi` CLI subcommand and
/// consumed by the CI drift check (openapi-typescript regeneration).
pub fn openapi_pretty_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI serializes to JSON")
}
