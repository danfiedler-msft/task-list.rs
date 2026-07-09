//! Task use-cases. Each is a plain `async fn` taking the ports it needs — no mediator.

use crate::dto::TaskDto;
use crate::error::ApplicationError;
use crate::ports::TaskRepository;
use tasklist_domain::UserId;

/// List the caller's tasks, mapped to transport DTOs.
///
/// This is the walking-skeleton use-case: it flows handler -> use-case -> `TaskRepository`
/// port -> adapter, proving the dependency-inversion seam end to end.
pub async fn list(
    repo: &dyn TaskRepository,
    owner: &UserId,
) -> Result<Vec<TaskDto>, ApplicationError> {
    let tasks = repo.list_for_owner(owner).await?;
    Ok(tasks.into_iter().map(TaskDto::from).collect())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use time::OffsetDateTime;

    use tasklist_domain::{Task, TaskId, TaskStatus, Title, UserId};

    use super::*;

    /// Hand-written in-crate fake of the repository port — proves the use-case runs
    /// against any `TaskRepository` implementation. Only `list`/`get` are exercised by
    /// these tests; `create`/`update` echo their input (the etag-conflict semantics are
    /// covered against the in-memory adapter in the `infrastructure` crate).
    #[derive(Default)]
    struct FakeRepo {
        tasks: Vec<Task>,
    }

    #[async_trait]
    impl TaskRepository for FakeRepo {
        async fn list_for_owner(&self, owner: &UserId) -> Result<Vec<Task>, ApplicationError> {
            Ok(self
                .tasks
                .iter()
                .filter(|t| &t.owner == owner)
                .cloned()
                .collect())
        }

        async fn get(&self, owner: &UserId, id: &TaskId) -> Result<Task, ApplicationError> {
            self.tasks
                .iter()
                .find(|t| &t.owner == owner && &t.id == id)
                .cloned()
                .ok_or(ApplicationError::NotFound)
        }

        async fn create(&self, task: &Task) -> Result<Task, ApplicationError> {
            Ok(task.clone())
        }

        async fn update(&self, task: &Task) -> Result<Task, ApplicationError> {
            Ok(task.clone())
        }
    }

    #[tokio::test]
    async fn list_returns_only_the_owners_tasks() {
        let owner = UserId::parse("auth0|me").unwrap();
        let other = UserId::parse("auth0|other").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let repo = FakeRepo {
            tasks: vec![
                Task::new(
                    TaskId::new(),
                    owner.clone(),
                    Title::parse("mine").unwrap(),
                    TaskStatus::Todo,
                    now,
                ),
                Task::new(
                    TaskId::new(),
                    other,
                    Title::parse("theirs").unwrap(),
                    TaskStatus::Done,
                    now,
                ),
            ],
        };

        let result = list(&repo, &owner).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "mine");
        assert_eq!(result[0].owner, "auth0|me");
    }

    #[tokio::test]
    async fn list_is_empty_when_owner_has_no_tasks() {
        let repo = FakeRepo::default();
        let owner = UserId::parse("auth0|nobody").unwrap();
        assert!(list(&repo, &owner).await.unwrap().is_empty());
    }
}
