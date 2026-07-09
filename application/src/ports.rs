use async_trait::async_trait;
use time::OffsetDateTime;

use tasklist_domain::{Task, UserId};

use crate::error::ApplicationError;

/// Persistence port for tasks. Implemented by an in-memory fake (Task 1) and, later, by a
/// Cosmos adapter — with no change to callers. Every read is scoped to a single owner.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// List all tasks owned by `owner`.
    async fn list_for_owner(&self, owner: &UserId) -> Result<Vec<Task>, ApplicationError>;
}

/// Time port, so use-cases and tests do not read the wall clock directly.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}
