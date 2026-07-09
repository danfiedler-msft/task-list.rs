//! Infrastructure layer: port adapters and runtime configuration.
//!
//! The only crate that will reference Azure SDKs (added in later tasks). For the walking
//! skeleton it supplies an in-memory `TaskRepository`, a `SystemClock`, and a config
//! loader with the `TASKLIST_ENV` seam.

mod clock;
mod config;
mod repository;

pub use clock::SystemClock;
pub use config::{AppConfig, Environment};
pub use repository::InMemoryTaskRepository;
