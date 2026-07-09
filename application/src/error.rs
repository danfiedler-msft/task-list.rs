use thiserror::Error;

use tasklist_domain::DomainError;

/// Errors surfaced by application use-cases. `api` maps these to RFC7807 responses.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// A domain invariant was violated (maps to 400).
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// The requested entity does not exist or is not owned by the caller (maps to 404).
    #[error("task not found")]
    NotFound,

    /// The backing store failed (maps to 500).
    #[error("repository error: {0}")]
    Repository(String),
}
