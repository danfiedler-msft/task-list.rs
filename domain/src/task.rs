use time::OffsetDateTime;

use crate::{TaskId, Title, UserId};

/// The lifecycle status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

/// A task owned by a single user. Constructed from already-validated newtypes, so any
/// `Task` value is guaranteed to satisfy the domain invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub owner: UserId,
    pub title: Title,
    pub status: TaskStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Task {
    /// Create a new task, stamping both timestamps with `now`.
    pub fn new(
        id: TaskId,
        owner: UserId,
        title: Title,
        status: TaskStatus,
        now: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            owner,
            title,
            status,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_stamps_matching_timestamps() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let task = Task::new(
            TaskId::new(),
            UserId::parse("auth0|owner").unwrap(),
            Title::parse("Write the skeleton").unwrap(),
            TaskStatus::Todo,
            now,
        );
        assert_eq!(task.created_at, now);
        assert_eq!(task.updated_at, now);
        assert_eq!(task.status, TaskStatus::Todo);
    }

    #[test]
    fn default_status_is_todo() {
        assert_eq!(TaskStatus::default(), TaskStatus::Todo);
    }
}
