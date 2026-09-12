use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use mprober_lib::{
    Error, cpu, hostname, kernel, load_average, memory, network, rtc_time, uptime, volume,
};
use serde::Serialize;
use tokio::{
    sync::{Notify, watch},
    time,
};

use super::{
    cpu_sample::{self, CpuThreadSnapshot},
    probes::{CgroupSummary, NetworkWithSpeed, SystemPressure, VolumeWithSpeed},
};

/// How long sampling keeps running after the last one-shot request, expressed in detect intervals.
const IDLE_INTERVALS: u32 = 3;

/// How old a sample may be before it is treated as left over from before a pause, in detect intervals.
const STALE_INTERVALS: u32 = 2;

/// How long a request waits for a fresh sample, in detect intervals.
///
/// A round takes a whole detect interval, and a round which fails publishes nothing while it waits to
/// try again, so without a bound a request would wait for ever on a machine whose probes never succeed.
const WAIT_INTERVALS: u32 = 3;

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub hostname:     String,
    pub kernel:       String,
    pub uptime:       uptime::Uptime,
    pub rtc_time:     chrono::NaiveDateTime,
    pub load_average: load_average::LoadAverage,
    pub cpus:         Vec<cpu::CPU>,
    pub cpus_stat:    Vec<f64>,
    pub cpu_threads:  Vec<CpuThreadSnapshot>,
    pub memory:       memory::Free,
    pub network:      Vec<NetworkWithSpeed>,
    pub volumes:      Vec<VolumeWithSpeed>,
    /// `None` when the kernel was built without `CONFIG_PSI` or PSI was disabled.
    pub pressure:     Option<SystemPressure>,
    /// `None` when this process does not run under cgroup v2.
    pub cgroup:       Option<CgroupSummary>,
}

/// A snapshot together with when it was taken, so that a reader can tell a current one from one left over from before a pause.
#[derive(Debug, Clone)]
pub struct Sample {
    taken_at:     Instant,
    pub snapshot: Arc<Snapshot>,
}

impl Sample {
    /// A running sampler publishes once per detect interval, so anything older than a couple of those was taken before sampling paused and says nothing about the machine now.
    #[inline]
    pub fn is_fresh(&self, detect_interval: Duration) -> bool {
        self.taken_at.elapsed() < detect_interval * STALE_INTERVALS
    }
}

#[derive(Debug)]
struct Shared {
    wake:         Notify,
    last_request: Mutex<Instant>,
}

/// A single background task samples for every client, so that N open pages still cost one sampling round.
///
/// Only the sender is kept here, and every receiver is created on demand, so that the receiver count
/// is exactly the number of readers waiting. Holding a receiver in this struct would count the
/// long-lived clones of the application state as readers, and sampling would never stop.
#[derive(Debug, Clone)]
pub struct Sampler {
    sender:          Arc<watch::Sender<Option<Sample>>>,
    shared:          Arc<Shared>,
    detect_interval: Duration,
}

impl Sampler {
    pub fn spawn(detect_interval: Duration) -> Self {
        let sender = Arc::new(watch::Sender::new(None));

        let shared = Arc::new(Shared {
            wake:         Notify::new(),
            // Sampling starts right away, so that the first request does not have to wait for a whole interval.
            last_request: Mutex::new(Instant::now()),
        });

        tokio::spawn(run(sender.clone(), shared.clone(), detect_interval));

        Sampler {
            sender,
            shared,
            detect_interval,
        }
    }

    /// Subscribe to every future sample. Sampling runs as long as at least one subscriber is alive.
    pub fn subscribe(&self) -> watch::Receiver<Option<Sample>> {
        let receiver = self.sender.subscribe();

        self.shared.wake.notify_one();

        receiver
    }

    /// Get the latest snapshot, waiting for a fresh one when sampling has just started or has just resumed.
    ///
    /// `None` once the wait has gone on for `WAIT_INTERVALS`, or when the sampler is gone for good.
    pub async fn latest(&self) -> Option<Arc<Snapshot>> {
        *self.shared.last_request.lock().unwrap() = Instant::now();

        // This receiver keeps the sampler awake for as long as the wait lasts.
        let mut receiver = self.sender.subscribe();

        self.shared.wake.notify_one();

        let wait = async {
            loop {
                // The age is checked here rather than cleared by the sampler, because the sampler
                // cannot clear it before this reader looks: it is woken by the same notification that
                // follows.
                if let Some(sample) = receiver.borrow_and_update().clone()
                    && sample.is_fresh(self.detect_interval)
                {
                    return Some(sample.snapshot);
                }

                if receiver.changed().await.is_err() {
                    return None;
                }
            }
        };

        time::timeout(self.detect_interval * WAIT_INTERVALS, wait).await.ok().flatten()
    }
}

async fn run(
    sender: Arc<watch::Sender<Option<Sample>>>,
    shared: Arc<Shared>,
    detect_interval: Duration,
) {
    let idle_timeout = detect_interval * IDLE_INTERVALS;

    loop {
        // Park while nobody is watching, so that an idle machine is not probed for nothing.
        while sender.receiver_count() == 0
            && shared.last_request.lock().unwrap().elapsed() >= idle_timeout
        {
            shared.wake.notified().await;
        }

        match tokio::task::spawn_blocking(move || sample(detect_interval)).await {
            Ok(Ok(snapshot)) => {
                sender.send_replace(Some(Sample {
                    taken_at: Instant::now(),
                    snapshot: Arc::new(snapshot),
                }));
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
    let (cpu_sample, network, volumes) = thread::scope(|scope| {
        let cpus_stat = scope.spawn(|| cpu_sample::sample(detect_interval));
        let network = scope.spawn(|| network::get_networks_with_speed(detect_interval));

        let volumes = volume::get_volumes_with_speed(detect_interval);

        (cpus_stat.join().unwrap(), network.join().unwrap(), volumes)
    });

    let cpu_sample = cpu_sample?;

    Ok(Snapshot {
        hostname:     hostname::get_hostname()?,
        kernel:       kernel::get_kernel_version()?,
        uptime:       uptime::get_uptime()?,
        rtc_time:     rtc_time::get_rtc_date_time()?,
        load_average: load_average::get_load_average()?,
        cpus:         cpu::get_cpus()?,
        cpus_stat:    cpu_sample.cpus_stat,
        cpu_threads:  cpu_sample.threads,
        memory:       memory::free()?,
        network:      NetworkWithSpeed::collect(network?),
        volumes:      VolumeWithSpeed::collect(volumes?),
        pressure:     SystemPressure::probe()?,
        cgroup:       CgroupSummary::probe()?,
    })
}
