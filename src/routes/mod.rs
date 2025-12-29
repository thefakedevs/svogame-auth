mod auth;
mod poll;
mod verification;
mod gamervii;

use axum::extract::State;
use crate::state::AppState;

pub use auth::*;
pub use poll::*;
pub use gamervii::*;
pub use verification::*;

pub type AxumAppState = State<std::sync::Arc<tokio::sync::RwLock<AppState>>>;
