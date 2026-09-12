use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use mprober_lib::{
    Error, cpu, hostname, kernel, load_average, memory, network, rtc_time, uptime, volume,
};
use serde::Serialize;

use super::{
    error::{ApiError, ApiResult},
    probes::{CgroupReport, SystemPressure},
    sampler::Snapshot,
    state::AppState,
};

/// Every probe reads files and some of them sleep, so none of them may run on a runtime thread.
async fn blocking<T, F>(probe: F) -> ApiResult<T>
where
    F: FnOnce() -> Result<T, Error> + Send + 'static,
    T: Send + 'static, {
    Ok(tokio::task::spawn_blocking(probe).await??)
}

#[derive(Debug, Serialize)]
pub struct Config {
    pub version:         &'static str,
    /// How often the sampler publishes a new snapshot, in seconds.
    pub detect_interval: f64,
}

pub async fn config(State(state): State<AppState>) -> Json<Config> {
    Json(Config {
        version:         env!("CARGO_PKG_VERSION"),
        detect_interval: state.detect_interval.as_secs_f64(),
    })
}

#[derive(Debug, Serialize)]
pub struct CpuReport {
    pub load_average: load_average::LoadAverage,
    pub cpus:         Vec<cpu::CPU>,
}

#[derive(Debug, Serialize)]
pub struct CpuDetectReport<'a> {
    pub load_average: &'a load_average::LoadAverage,
    pub cpus:         &'a [cpu::CPU],
    pub cpus_stat:    &'a [f64],
}

pub async fn hostname() -> ApiResult<Json<String>> {
    Ok(Json(blocking(hostname::get_hostname).await?))
}

pub async fn kernel() -> ApiResult<Json<String>> {
    Ok(Json(blocking(kernel::get_kernel_version).await?))
}

pub async fn uptime() -> ApiResult<Json<uptime::Uptime>> {
    Ok(Json(blocking(uptime::get_uptime).await?))
}

pub async fn time() -> ApiResult<Json<chrono::NaiveDateTime>> {
    Ok(Json(blocking(rtc_time::get_rtc_date_time).await?))
}

pub async fn cpu() -> ApiResult<Json<CpuReport>> {
    let report = blocking(|| {
        Ok(CpuReport {
            load_average: load_average::get_load_average()?,
            cpus:         cpu::get_cpus()?,
        })
    })
    .await?;

    Ok(Json(report))
}

pub async fn memory() -> ApiResult<Json<memory::Free>> {
    Ok(Json(blocking(memory::free).await?))
}

pub async fn network() -> ApiResult<Json<Vec<network::Network>>> {
    Ok(Json(blocking(network::get_networks).await?))
}

pub async fn volume() -> ApiResult<Json<Vec<volume::Volume>>> {
    Ok(Json(blocking(volume::get_volumes).await?))
}

pub async fn pressure() -> ApiResult<Json<SystemPressure>> {
    let pressure = blocking(SystemPressure::probe).await?;

    pressure
        .map(Json)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "this kernel does not provide PSI"))
}

pub async fn cgroup() -> ApiResult<Json<CgroupReport>> {
    let report = blocking(CgroupReport::probe).await?;

    report.map(Json).ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, "this process does not run under cgroup v2")
    })
}

/// Like `all`, these serialize straight out of the `Arc`, since cloning would copy every list they name for each request.
pub async fn cpu_detect(State(state): State<AppState>) -> ApiResult<Response> {
    let snapshot = latest(&state).await?;

    let report = CpuDetectReport {
        load_average: &snapshot.load_average,
        cpus:         &snapshot.cpus,
        cpus_stat:    &snapshot.cpus_stat,
    };

    Ok(Json(report).into_response())
}

pub async fn network_detect(State(state): State<AppState>) -> ApiResult<Response> {
    let snapshot = latest(&state).await?;

    Ok(Json(&snapshot.network).into_response())
}

pub async fn volume_detect(State(state): State<AppState>) -> ApiResult<Response> {
    let snapshot = latest(&state).await?;

    Ok(Json(&snapshot.volumes).into_response())
}

/// The snapshot is serialized straight out of its `Arc`, since cloning it would copy every list it holds for each request.
pub async fn all(State(state): State<AppState>) -> ApiResult<Response> {
    let snapshot = latest(&state).await?;

    Ok(Json(&*snapshot).into_response())
}

/// The sampler publishes a snapshot every round, so nothing here means it has stopped for good or its probes keep failing.
async fn latest(state: &AppState) -> ApiResult<Arc<Snapshot>> {
    state
        .sampler
        .latest()
        .await
        .ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "the sampler is not running"))
}
