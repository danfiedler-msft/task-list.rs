//! Application layer: use-cases, port traits, DTOs and application errors.
//!
//! Depends only on `domain`. Ports (`TaskRepository`, `Clock`) are pure Rust traits with
//! no Azure or web-framework types; adapters live in `infrastructure`, the composition
//! root in `api`. This is the seam that keeps the (preview) persistence SDK swappable.

pub mod dto;
pub mod error;
pub mod ports;
pub mod tasks;

pub use error::ApplicationError;
pub use ports::{Clock, TaskRepository};
