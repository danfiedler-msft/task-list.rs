use std::sync::RwLock;

use async_trait::async_trait;

use tasklist_application::error::ApplicationError;
use tasklist_application::ports::TaskRepository;
use tasklist_domain::{Task, UserId};

/// In-memory `TaskRepository` used by the walking skeleton and by tests. Swapped for the
/// Cosmos adapter in a later task with no change to `domain`/`application`.
#[derive(Default)]
pub struct InMemoryTaskRepository {
    tasks: RwLock<Vec<Task>>,
}

impl InMemoryTaskRepository {
    /// An empty repository.
    pub fn new() -> Self {
        Self::default()
    }

    /// A repository pre-populated with `tasks` (used to seed the skeleton's demo data).
    pub fn seeded(tasks: Vec<Task>) -> Self {
        Self {
            tasks: RwLock::new(tasks),
        }
    }
}

#[async_trait]
impl TaskRepository for InMemoryTaskRepository {
    async fn list_for_owner(&self, owner: &UserId) -> Result<Vec<Task>, ApplicationError> {
        let guard = self
            .tasks
            .read()
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;
        Ok(guard
            .iter()
            .filter(|t| &t.owner == owner)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use tasklist_domain::{Task, TaskId, TaskStatus, Title, UserId};

    use super::*;

    fn task_for(owner: &UserId, title: &str) -> Task {
        Task::new(
            TaskId::new(),
            owner.clone(),
            Title::parse(title).unwrap(),
            TaskStatus::Todo,
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn list_for_owner_filters_by_owner() {
        let me = UserId::parse("auth0|me").unwrap();
        let other = UserId::parse("auth0|other").unwrap();
        let repo = InMemoryTaskRepository::seeded(vec![task_for(&me, "a"), task_for(&other, "b")]);

        let mine = repo.list_for_owner(&me).await.unwrap();

        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].owner, me);
    }

    #[tokio::test]
    async fn empty_repository_lists_nothing() {
        let repo = InMemoryTaskRepository::new();
        let owner = UserId::parse("auth0|me").unwrap();
        assert!(repo.list_for_owner(&owner).await.unwrap().is_empty());
    }
}
