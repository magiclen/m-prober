use std::{sync::Arc, time::Duration};

use tokio::sync::watch;

use super::{auth::Auth, sampler::Sampler};

/// Tells the request handlers that the server has begun to stop.
///
/// An event stream never ends by itself, and a graceful shutdown waits for every open connection, so
/// without this the process would keep running for as long as one page stayed open.
#[derive(Debug, Clone)]
pub struct Shutdown(watch::Receiver<bool>);

impl Shutdown {
    pub fn channel() -> (watch::Sender<bool>, Self) {
        let (sender, receiver) = watch::channel(false);

        (sender, Shutdown(receiver))
    }

    #[inline]
    pub fn from_receiver(receiver: watch::Receiver<bool>) -> Self {
        Shutdown(receiver)
    }

    /// Resolves once the shutdown has started, and never if the sender is gone.
    pub async fn started(mut self) {
        while !*self.0.borrow_and_update() {
            if self.0.changed().await.is_err() {
                return std::future::pending().await;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub detect_interval: Duration,
    pub auth:            Arc<Auth>,
    pub sampler:         Sampler,
    pub shutdown:        Shutdown,
}

impl AppState {
    pub fn new(detect_interval: Duration, auth_key: Option<String>, shutdown: Shutdown) -> Self {
        AppState {
            detect_interval,
            auth: Arc::new(Auth::new(auth_key)),
            sampler: Sampler::spawn(detect_interval),
            shutdown,
        }
    }
}
