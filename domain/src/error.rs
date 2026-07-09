use thiserror::Error;

/// Errors raised when a domain invariant is violated at construction time.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("user id must not be empty")]
    EmptyUserId,

    #[error("title must be between 1 and 200 characters")]
    InvalidTitle,
}
