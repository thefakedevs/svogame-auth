use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};

const AUTH_RESULT_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Debug)]
pub struct AuthCompleteEvent {
    pub poll_id: String,
    pub result: AuthPollResult,
}

#[derive(Clone, Debug)]
struct CachedAuthResult {
    result: AuthPollResult,
    created_at: Instant,
}

#[derive(Clone, Debug)]
pub enum AuthPollResult {
    Pending,
    Success {
        access_token: String,
        user_id: String,
        username: String,
        avatar_url: String,
    },
    Error {
        message: String,
    },
    Expired,
}

pub struct AuthRuntime {
    events_tx: broadcast::Sender<AuthCompleteEvent>,
    results_cache: Arc<RwLock<HashMap<String, CachedAuthResult>>>,
}

impl AuthRuntime {
    pub fn new() -> Self {
        let (events_tx, _) = broadcast::channel(256);

        Self {
            events_tx,
            results_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<AuthCompleteEvent> {
        self.events_tx.subscribe()
    }

    pub fn notify_complete(&self, poll_id: String, result: AuthPollResult) {
        let _ = self.events_tx.send(AuthCompleteEvent {
            poll_id: poll_id.clone(),
            result: result.clone(),
        });

        let cache = self.results_cache.clone();
        tokio::spawn(async move {
            let mut cache_guard = cache.write().await;
            cache_guard.insert(
                poll_id,
                CachedAuthResult {
                    result,
                    created_at: Instant::now(),
                },
            );
        });
    }

    pub async fn get_cached_result(&self, poll_id: &str) -> Option<AuthPollResult> {
        let cache = self.results_cache.read().await;
        cache.get(poll_id).map(|cached| cached.result.clone())
    }

    pub async fn cleanup_cache(&self) {
        let mut cache = self.results_cache.write().await;
        let now = Instant::now();
        cache.retain(|_, cached| now.duration_since(cached.created_at) < AUTH_RESULT_TTL);
    }
}
