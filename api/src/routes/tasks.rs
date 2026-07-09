use axum::extract::State;
use axum::Json;

use tasklist_application::dto::TaskDto;

use crate::error::ApiError;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/tasks",
    tag = "tasks",
    responses((status = 200, description = "The caller's tasks", body = [TaskDto]))
)]
pub async fn list_tasks(State(state): State<AppState>) -> Result<Json<Vec<TaskDto>>, ApiError> {
    // handler -> use-case -> TaskRepository port -> adapter: the dependency-inversion path.
    let tasks = tasklist_application::tasks::list(state.repo.as_ref(), &state.demo_owner).await?;
    Ok(Json(tasks))
}
