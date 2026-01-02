mod auth;
mod poll;
mod verification;
mod gamervii;
mod user;

use axum::extract::State;
use crate::state::AppState;

pub use auth::*;
pub use poll::*;
pub use gamervii::*;
pub use verification::*;
pub use user::*;

pub type AxumAppState = State<std::sync::Arc<tokio::sync::RwLock<AppState>>>;
