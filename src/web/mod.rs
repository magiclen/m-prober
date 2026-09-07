mod api;
mod auth;
mod error;
mod probes;
mod sampler;
mod state;
mod static_files;
mod stream;
#[cfg(test)]
mod tests;

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use axum::{
    Json, Router,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use serde_json::json;
use state::AppState;
use tokio::{net::TcpListener, signal};

pub fn serve(
    monitor: Duration,
    address: IpAddr,
    listen_port: u16,
    auth_key: Option<String>,
    only_api: bool,
) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(async move {
        let state = AppState::new(monitor, auth_key);
        let app = router(state, only_api);

        let listener = TcpListener::bind(SocketAddr::new(address, listen_port)).await?;

        println!("mprober is listening on http://{}", listener.local_addr()?);

        axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

        Ok(())
    })
}

fn router(state: AppState, only_api: bool) -> Router {
    // Logging in has to stay reachable without being logged in, and so does what the login page shows.
    let public = Router::new()
        .route("/config", get(api::config))
        .route("/auth", get(auth::status).post(auth::login))
        .route("/logout", post(auth::logout));

    let protected = Router::new()
        .route("/hostname", get(api::hostname))
        .route("/kernel", get(api::kernel))
        .route("/uptime", get(api::uptime))
        .route("/time", get(api::time))
        .route("/cpu", get(api::cpu))
        .route("/memory", get(api::memory))
        .route("/network", get(api::network))
        .route("/volume", get(api::volume))
        .route("/pressure", get(api::pressure))
        .route("/cgroup", get(api::cgroup))
        .route("/cpu-detect", get(api::cpu_detect))
        .route("/network-detect", get(api::network_detect))
        .route("/volume-detect", get(api::volume_detect))
        .route("/all", get(api::all))
        .route("/all/stream", get(stream::all))
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_auth));

    let api = public.merge(protected).fallback(api_not_found);

    let app = Router::new().nest("/api", api);

    let app = if only_api { app } else { app.fallback(static_files::serve) };

    app.with_state(state)
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "no such API" })))
}

async fn shutdown_signal() {
    let interrupt = async {
        signal::ctrl_c().await.expect("cannot listen for SIGINT");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("cannot listen for SIGTERM")
            .recv()
            .await;
    };

    tokio::select! {
        () = interrupt => (),
        () = terminate => (),
    }
}
