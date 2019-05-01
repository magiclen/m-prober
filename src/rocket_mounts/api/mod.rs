use std::thread;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Instant, Duration};

use crate::rocket::{Rocket, http::Status};
use crate::rocket_simple_authorization::SimpleAuthorization;
use crate::rocket_cache_response::CacheResponse;
use crate::rocket_json_response::{JSONResponse, json_gettext::{serde_json::Value, JSONGetTextValue}};

use crate::kernel;
use crate::cpu_info::{CPU, CPUStat};
use crate::network::NetworkWithSpeed;
use crate::disk::DiskWithSpeed;

static mut CPUS_STAT_DOING: AtomicBool = AtomicBool::new(false);
static mut NETWORK_STAT_DOING: AtomicBool = AtomicBool::new(false);
static mut DISKS_STAT_DOING: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref CPUS_STAT_LATEST_DETECT: Mutex<Option<Instant>> = {
        Mutex::new(Some(Instant::now()))
    };

    static ref NETWORK_LATEST_DETECT: Mutex<Option<Instant>> = {
        Mutex::new(Some(Instant::now()))
    };

    static ref DISKS_STAT_LATEST_DETECT: Mutex<Option<Instant>> = {
        Mutex::new(Some(Instant::now()))
    };
}

lazy_static! {
    static ref CPUS_STAT: Mutex<Option<f64>> = {
        Mutex::new(Some(0f64))
    };

    static ref NETWORK_STAT: Mutex<Option<Vec<NetworkWithSpeed>>> = {
        Mutex::new(Some(vec![]))
    };

    static ref DISKS_STAT: Mutex<Option<Vec<DiskWithSpeed>>> = {
        Mutex::new(Some(vec![]))
    };
}

pub struct Auth;

impl SimpleAuthorization for Auth {
    #[inline]
    fn has_authority<S: AsRef<str>>(key: Option<S>) -> bool {
        match unsafe { super::AUTH_KEY.as_ref() } {
            Some(auth_key) => {
                match key {
                    Some(key) => key.as_ref().eq(auth_key),
                    None => false
                }
            }
            None => true
        }
    }

    #[inline]
    fn create_auth<S: AsRef<str>>(_key: Option<S>) -> Auth {
        Auth
    }
}

authorizer!(Auth);

#[inline]
fn detect_all_sleep() {
    let now = Instant::now();

    let latest = CPUS_STAT_LATEST_DETECT.lock().unwrap().unwrap().max(NETWORK_LATEST_DETECT.lock().unwrap().unwrap()).max(DISKS_STAT_LATEST_DETECT.lock().unwrap().unwrap());

    if now > latest {
        let d: Duration = now - latest;
        let detect_interval = unsafe { super::DETECT_INTERVAL };

        if d < detect_interval {
            thread::sleep(detect_interval - d);
        }
    }
}

fn fetch_cpus_stat() {
    if !unsafe { CPUS_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        CPUS_STAT_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let cpus_stat = CPUStat::get_average_percentage(unsafe { super::DETECT_INTERVAL }).unwrap();

            CPUS_STAT.lock().unwrap().replace(cpus_stat);

            unsafe { CPUS_STAT_DOING.swap(false, Ordering::Relaxed); }
        });
    }
}

fn fetch_network_stat() {
    if !unsafe { NETWORK_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        NETWORK_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let network_stat = NetworkWithSpeed::get_networks_with_speed(unsafe { super::DETECT_INTERVAL }).unwrap();

            NETWORK_STAT.lock().unwrap().replace(network_stat);

            unsafe { NETWORK_STAT_DOING.swap(false, Ordering::Relaxed); }
        });
    }
}

fn fetch_disks_stat() {
    if !unsafe { DISKS_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        DISKS_STAT_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let disk_stat = DiskWithSpeed::get_disks_with_speed(unsafe { super::DETECT_INTERVAL }).unwrap();

            DISKS_STAT.lock().unwrap().replace(disk_stat);

            unsafe { DISKS_STAT_DOING.swap(false, Ordering::Relaxed); }
        });
    }
}

#[get("/kernel")]
fn kernel(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::JSONValue(Value::String(kernel::get_kernel_version().unwrap()))))
}

#[get("/kernel", rank = 2)]
fn kernel_401() -> Status {
    Status::Unauthorized
}

#[get("/monitor")]
fn monitor(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    fetch_cpus_stat();
    fetch_network_stat();
    fetch_disks_stat();

    detect_all_sleep();

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::Str("Test")))
}

#[get("/monitor", rank = 2)]
fn monitor_401() -> Status {
    Status::Unauthorized
}

pub fn mounts(rocket: Rocket) -> Rocket {
    rocket
        .mount("/api", routes![kernel, kernel_401])
        .mount("/api", routes![monitor, monitor_401])
}