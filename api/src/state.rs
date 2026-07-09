use std::sync::Arc;

use tasklist_application::ports::{Clock, TaskRepository};
use tasklist_domain::UserId;

/// Shared application state: the ports (as trait objects) plus the current demo owner.
/// Constructor injection happens in the composition root (`seed_demo_state`), so handlers
/// depend on the port traits, never on a concrete adapter — the DIP proof.
#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn TaskRepository>,
    pub clock: Arc<dyn Clock>,
    pub demo_owner: UserId,
}
