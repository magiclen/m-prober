use std::thread;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Instant, Duration};

use crate::rocket::{Rocket, http::Status};
use crate::rocket_simple_authorization::SimpleAuthorization;
use crate::rocket_cache_response::CacheResponse;
use crate::rocket_json_response::{JSONResponse, json_gettext::{serde_json::Value, JSONGetTextValue}};

use crate::byte_unit::{Byte, ByteUnit};

use crate::hostname;
use crate::time::{self, RTCDateTime};
use crate::kernel;
use crate::load_average::LoadAverage;
use crate::cpu_info::{CPU, CPUStat};
use crate::free::Free;
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
    fn has_authority<S: AsRef<str>>(key: Option<S>) -> Option<Option<String>> {
        match unsafe { super::AUTH_KEY.as_ref() } {
            Some(auth_key) => {
                match key {
                    Some(key) => if key.as_ref().eq(auth_key) {
                        Some(None)
                    } else {
                        None
                    },
                    None => None
                }
            }
            None => Some(None)
        }
    }

    #[inline]
    fn create_auth(_key: Option<String>) -> Auth {
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

    let load_average = LoadAverage::get_load_average().unwrap();

    let cpus = CPU::get_cpus().unwrap();

    let memory = Free::get_free().unwrap();

    let hostname = hostname::get_hostname().unwrap();

    let uptime = time::get_uptime().unwrap();

    let time = RTCDateTime::get_rtc_date_time().unwrap();

    let cpus_stat = CPUS_STAT.lock().unwrap().unwrap();

    let uptime_string = time::format_duration(uptime);

    let json_cpus = {
        let mut json_cpus = Vec::with_capacity(cpus.len());

        for cpu in cpus {
            let cpus_mhz_len = cpu.cpus_mhz.len();

            let mhz = cpu.cpus_mhz.iter().sum::<f64>() / cpus_mhz_len as f64;
            let mhz_string = {
                let adjusted_byte = Byte::from_unit(mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                format!("{:.2} {}Hz", adjusted_byte.get_value(), &adjusted_byte.get_unit().as_str()[..1])
            };

            json_cpus.push(json!({
                "model_name": cpu.model_name,
                "cores": cpu.cpu_cores,
                "threads": cpu.siblings,
                "mhz": {
                    "value": mhz,
                    "text": mhz_string
                },
            }));
        }

        json_cpus
    };

    let memory_buff_and_cache = memory.mem.buffers + memory.mem.cache;

    let memory_total_string = Byte::from_bytes(memory.mem.total as u128).get_appropriate_unit(true).to_string();
    let memory_used_string = Byte::from_bytes(memory.mem.used as u128).get_appropriate_unit(true).to_string();
    let memory_buff_and_cache_string = Byte::from_bytes(memory_buff_and_cache as u128).get_appropriate_unit(true).to_string();

    let swap_total_string = Byte::from_bytes(memory.swap.total as u128).get_appropriate_unit(true).to_string();
    let swap_used_string = Byte::from_bytes(memory.swap.used as u128).get_appropriate_unit(true).to_string();
    let swap_cache_string = Byte::from_bytes(memory.swap.cache as u128).get_appropriate_unit(true).to_string();

    let json_network = {
        let network_stat = NETWORK_STAT.lock().unwrap();

        let network_stat: &[NetworkWithSpeed] = network_stat.as_ref().unwrap();

        let mut json_network = Vec::with_capacity(network_stat.len());

        for network_with_speed in network_stat {
            let upload_total_string = Byte::from_bytes(network_with_speed.network.transmit_bytes as u128).get_appropriate_unit(false).to_string();
            let download_total_string = Byte::from_bytes(network_with_speed.network.receive_bytes as u128).get_appropriate_unit(false).to_string();

            let upload_rate_string = {
                let mut s = Byte::from_bytes(network_with_speed.speed.transmit as u128).get_appropriate_unit(false).to_string();

                s.push_str("/s");

                s
            };

            let download_rate_string = {
                let mut s = Byte::from_bytes(network_with_speed.speed.receive as u128).get_appropriate_unit(false).to_string();

                s.push_str("/s");

                s
            };

            json_network.push(json!({
                "interface": network_with_speed.network.interface,
                "upload_total": {
                    "value": network_with_speed.network.transmit_bytes,
                    "text": upload_total_string
                },
                "download_total": {
                    "value": network_with_speed.network.receive_bytes,
                    "text": download_total_string
                },
                "upload_rate": {
                    "value": network_with_speed.speed.transmit,
                    "text": upload_rate_string
                },
                "download_rate": {
                    "value": network_with_speed.speed.receive,
                    "text": download_rate_string
                },
            }));
        }

        json_network
    };

    let json_disks = {
        let disks_stat = DISKS_STAT.lock().unwrap();

        let disks_stat: &[DiskWithSpeed] = disks_stat.as_ref().unwrap();

        let mut json_disks = Vec::with_capacity(disks_stat.len());

        for disk_with_speed in disks_stat {
            let read_total_string = Byte::from_bytes(disk_with_speed.disk.read_bytes as u128).get_appropriate_unit(false).to_string();
            let write_total_string = Byte::from_bytes(disk_with_speed.disk.write_bytes as u128).get_appropriate_unit(false).to_string();

            let read_rate_string = {
                let mut s = Byte::from_bytes(disk_with_speed.speed.read as u128).get_appropriate_unit(false).to_string();

                s.push_str("/s");

                s
            };

            let download_rate_string = {
                let mut s = Byte::from_bytes(disk_with_speed.speed.write as u128).get_appropriate_unit(false).to_string();

                s.push_str("/s");

                s
            };

            json_disks.push(json!({
                "device": disk_with_speed.disk.device,
                "read_total": {
                    "value": disk_with_speed.disk.read_bytes,
                    "text": read_total_string
                },
                "write_total": {
                    "value": disk_with_speed.disk.write_bytes,
                    "text": write_total_string
                },
                "read_rate": {
                    "value": disk_with_speed.speed.read,
                    "text": read_rate_string
                },
                "write_rate": {
                    "value": disk_with_speed.speed.write,
                    "text": download_rate_string
                },
                "mount_points": disk_with_speed.disk.points
            }));
        }

        json_disks
    };

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::JSONValue(json!({
        "hostname": hostname,
        "uptime": {
            "seconds": uptime.as_secs(),
            "text": uptime_string
        },
        "rtc_time": format!("{} {}", time.rtc_date, time.rtc_time),
        "load_average": {
            "one": load_average.one,
            "five": load_average.five,
            "fifteen": load_average.fifteen
        },
        "cpu": cpus_stat,
        "cpus": json_cpus,
        "memory": {
            "total": {
                "value": memory.mem.total,
                "text": memory_total_string
            },
            "used": {
                "value": memory.mem.used,
                "text": memory_used_string,
            },
            "buff/cache": {
                "value": memory.mem.buffers + memory.mem.cache,
                "text": memory_buff_and_cache_string
            },
        },
        "swap": {
            "total": {
                "value": memory.swap.total,
                "text": swap_total_string
            },
            "used": {
                "value": memory.swap.used,
                "text": swap_used_string
            },
            "cache": {
                "value": memory.swap.cache,
                "text": swap_cache_string
            },
        },
        "network": json_network,
        "disks": json_disks,
    }))))
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