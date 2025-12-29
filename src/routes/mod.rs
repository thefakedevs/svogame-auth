mod auth;
mod verification;

use axum::extract::State;
use crate::state::AppState;

pub use auth::*;
pub use verification::*;

pub type AxumAppState = State<std::sync::Arc<tokio::sync::RwLock<AppState>>>;
