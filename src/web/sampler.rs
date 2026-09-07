use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use mprober_lib::{
    Error, cpu, hostname, kernel, load_average, memory, network, rtc_time, uptime, volume,
};
use serde::Serialize;
use tokio::sync::{Notify, watch};

use super::probes::{CgroupSummary, NetworkWithSpeed, SystemPressure, VolumeWithSpeed};

/// How long sampling keeps running after the last one-shot request, expressed in detect intervals.
const IDLE_INTERVALS: u32 = 3;

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub hostname:     String,
    pub kernel:       String,
    pub uptime:       uptime::Uptime,
    pub rtc_time:     chrono::NaiveDateTime,
    pub load_average: load_average::LoadAverage,
    pub cpus:         Vec<cpu::CPU>,
    pub cpus_stat:    Vec<f64>,
    pub memory:       memory::Free,
    pub network:      Vec<NetworkWithSpeed>,
    pub volumes:      Vec<VolumeWithSpeed>,
    /// `None` when the kernel was built without `CONFIG_PSI` or PSI was disabled.
    pub pressure:     Option<SystemPressure>,
    /// `None` when this process does not run under cgroup v2.
    pub cgroup:       Option<CgroupSummary>,
}

#[derive(Debug)]
struct Shared {
    wake:         Notify,
    last_request: Mutex<Instant>,
}

/// A single background task samples for every client, so that N open pages still cost one sampling round.
#[derive(Debug, Clone)]
pub struct Sampler {
    receiver: watch::Receiver<Option<Arc<Snapshot>>>,
    shared:   Arc<Shared>,
}

impl Sampler {
    pub fn spawn(detect_interval: Duration) -> Self {
        let (sender, receiver) = watch::channel(None);

        let shared = Arc::new(Shared {
            wake:         Notify::new(),
            // Sampling starts right away, so that the first request does not have to wait for a whole interval.
            last_request: Mutex::new(Instant::now()),
        });

        let sampler = Sampler {
            receiver,
            shared: shared.clone(),
        };

        tokio::spawn(run(sender, shared, detect_interval));

        sampler
    }

    /// Subscribe to every future snapshot. Sampling runs as long as at least one subscriber is alive.
    pub fn subscribe(&self) -> watch::Receiver<Option<Arc<Snapshot>>> {
        let receiver = self.receiver.clone();

        self.shared.wake.notify_one();

        receiver
    }

    /// Get the latest snapshot, waiting for the first one when sampling has just started.
    pub async fn latest(&self) -> Option<Arc<Snapshot>> {
        *self.shared.last_request.lock().unwrap() = Instant::now();
        self.shared.wake.notify_one();

        let mut receiver = self.receiver.clone();

        loop {
            if let Some(snapshot) = receiver.borrow_and_update().clone() {
                return Some(snapshot);
            }

            if receiver.changed().await.is_err() {
                return None;
            }
        }
    }
}

async fn run(
    sender: watch::Sender<Option<Arc<Snapshot>>>,
    shared: Arc<Shared>,
    detect_interval: Duration,
) {
    let idle_timeout = detect_interval * IDLE_INTERVALS;

    loop {
        // Park while nobody is watching, so that an idle machine is not probed for nothing.
        while sender.receiver_count() <= 1
            && shared.last_request.lock().unwrap().elapsed() >= idle_timeout
        {
            shared.wake.notified().await;
        }

        match tokio::task::spawn_blocking(move || sample(detect_interval)).await {
            Ok(Ok(snapshot)) => {
                sender.send_replace(Some(Arc::new(snapshot)));
            },
            Ok(Err(error)) => {
                // Keep the previous snapshot and try again, since a probe can fail while a device is being removed.
                eprintln!("mprober: cannot sample the system: {error}");

                tokio::time::sleep(detect_interval).await;
            },
            Err(error) => {
                eprintln!("mprober: the sampling task failed: {error}");

                tokio::time::sleep(detect_interval).await;
            },
        }
    }
}

fn sample(detect_interval: Duration) -> Result<Snapshot, Error> {
    // Each of these three probes sleeps for the whole interval, so they have to run at the same time.
    let (cpus_stat, network, volumes) = thread::scope(|scope| {
        let cpus_stat =
            scope.spawn(|| cpu::get_all_cpu_utilization_in_percentage(true, detect_interval));
        let network = scope.spawn(|| network::get_networks_with_speed(detect_interval));

        let volumes = volume::get_volumes_with_speed(detect_interval);

        (cpus_stat.join().unwrap(), network.join().unwrap(), volumes)
    });

    Ok(Snapshot {
        hostname:     hostname::get_hostname()?,
        kernel:       kernel::get_kernel_version()?,
        uptime:       uptime::get_uptime()?,
        rtc_time:     rtc_time::get_rtc_date_time()?,
        load_average: load_average::get_load_average()?,
        cpus:         cpu::get_cpus()?,
        cpus_stat:    cpus_stat?,
        memory:       memory::free()?,
        network:      NetworkWithSpeed::collect(network?),
        volumes:      VolumeWithSpeed::collect(volumes?),
        pressure:     SystemPressure::probe()?,
        cgroup:       CgroupSummary::probe()?,
    })
}
