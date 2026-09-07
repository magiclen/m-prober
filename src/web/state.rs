use std::{sync::Arc, time::Duration};

use super::{auth::Auth, sampler::Sampler};

#[derive(Debug, Clone)]
pub struct AppState {
    pub detect_interval: Duration,
    pub auth:            Arc<Auth>,
    pub sampler:         Sampler,
}

impl AppState {
    pub fn new(detect_interval: Duration, auth_key: Option<String>) -> Self {
        AppState {
            detect_interval,
            auth: Arc::new(Auth::new(auth_key)),
            sampler: Sampler::spawn(detect_interval),
        }
    }
}
