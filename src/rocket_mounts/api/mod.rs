use std::collections::linked_list::LinkedList;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::rocket::{http::Status, request::Request, Rocket, State};
use crate::rocket_cache_response::CacheResponse;
use crate::rocket_json_response::{json_gettext::JSONGetTextValue, JSONResponse};
use crate::rocket_simple_authorization::SimpleAuthorization;

use crate::byte_unit::{Byte, ByteUnit};

use crate::cpu_info::{CPUStat, CPU};
use crate::free::Free;
use crate::hostname;
use crate::kernel;
use crate::load_average::LoadAverage;
use crate::network::NetworkWithSpeed;
use crate::time::{self, RTCDateTime};
use crate::volume::{Volume, VolumeWithSpeed};

static mut CPUS_STAT_DOING: AtomicBool = AtomicBool::new(false);
static mut NETWORK_STAT_DOING: AtomicBool = AtomicBool::new(false);
static mut VOLUMES_STAT_DOING: AtomicBool = AtomicBool::new(false);

static DELAY_DURATION: Duration = Duration::from_millis(33);

lazy_static! {
    static ref CPUS_STAT_LATEST_DETECT: Mutex<Option<Instant>> =
        { Mutex::new(Some(Instant::now())) };
    static ref NETWORK_STAT_LATEST_DETECT: Mutex<Option<Instant>> =
        { Mutex::new(Some(Instant::now())) };
    static ref VOLUMES_STAT_LATEST_DETECT: Mutex<Option<Instant>> =
        { Mutex::new(Some(Instant::now())) };
}

lazy_static! {
    static ref CPUS_STAT: Mutex<Option<Vec<f64>>> = { Mutex::new(None) };
    static ref NETWORK_STAT: Mutex<Option<Vec<NetworkWithSpeed>>> = { Mutex::new(None) };
    static ref VOLUMES_STAT: Mutex<Option<Vec<VolumeWithSpeed>>> = { Mutex::new(None) };
}

pub struct Auth;

impl<'a, 'r> SimpleAuthorization<'a, 'r> for Auth {
    fn authorizing(request: &'a Request<'r>, authorization: Option<&'a str>) -> Option<Self> {
        let auth_key = request.guard::<State<super::AuthKey>>().unwrap();

        match auth_key.get_value() {
            Some(auth_key) => {
                match authorization {
                    Some(authorization) => {
                        if authorization.eq(auth_key) {
                            Some(Auth)
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            None => Some(Auth),
        }
    }
}

authorizer!(Auth);

#[inline]
fn detect_cpus_stat_sleep(detect_interval: Duration, strict: bool) {
    let now = Instant::now();

    let latest = CPUS_STAT_LATEST_DETECT.lock().unwrap().unwrap();

    if now > latest {
        let d: Duration = now - latest;
        let detect_interval = detect_interval + DELAY_DURATION;

        if d < detect_interval {
            thread::sleep(detect_interval - d);
        }
    }

    if strict {
        loop {
            let cont = CPUS_STAT.lock().unwrap().is_none();

            if cont {
                thread::sleep(DELAY_DURATION);
            } else {
                break;
            }
        }
    }
}

#[inline]
fn detect_network_stat_sleep(detect_interval: Duration, strict: bool) {
    let now = Instant::now();

    let latest = NETWORK_STAT_LATEST_DETECT.lock().unwrap().unwrap();

    if now > latest {
        let d: Duration = now - latest;
        let detect_interval = detect_interval + DELAY_DURATION;

        if d < detect_interval {
            thread::sleep(detect_interval - d);
        }
    }

    if strict {
        loop {
            let cont = NETWORK_STAT.lock().unwrap().is_none();

            if cont {
                thread::sleep(DELAY_DURATION);
            } else {
                break;
            }
        }
    }
}

#[inline]
fn detect_volumes_stat_sleep(detect_interval: Duration, strict: bool) {
    let now = Instant::now();

    let latest = VOLUMES_STAT_LATEST_DETECT.lock().unwrap().unwrap();

    if now > latest {
        let d: Duration = now - latest;
        let detect_interval = detect_interval + DELAY_DURATION;

        if d < detect_interval {
            thread::sleep(detect_interval - d);
        }
    }

    if strict {
        loop {
            let cont = VOLUMES_STAT.lock().unwrap().is_none();

            if cont {
                thread::sleep(DELAY_DURATION);
            } else {
                break;
            }
        }
    }
}

#[inline]
fn detect_all_sleep(detect_interval: Duration, strict: bool) {
    let now = Instant::now();

    let latest = CPUS_STAT_LATEST_DETECT
        .lock()
        .unwrap()
        .unwrap()
        .max(NETWORK_STAT_LATEST_DETECT.lock().unwrap().unwrap())
        .max(VOLUMES_STAT_LATEST_DETECT.lock().unwrap().unwrap());

    if now > latest {
        let d: Duration = now - latest;
        let detect_interval = detect_interval + DELAY_DURATION;

        if d < detect_interval {
            thread::sleep(detect_interval - d);
        }
    }

    if strict {
        loop {
            let cont = CPUS_STAT.lock().unwrap().is_none()
                || NETWORK_STAT.lock().unwrap().is_none()
                || VOLUMES_STAT.lock().unwrap().is_none();

            if cont {
                thread::sleep(DELAY_DURATION);
            } else {
                break;
            }
        }
    }
}

fn fetch_cpus_stat(detect_interval: Duration) {
    if !unsafe { CPUS_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        CPUS_STAT_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let cpus_stat = CPUStat::get_all_percentage(true, detect_interval).unwrap();

            CPUS_STAT.lock().unwrap().replace(cpus_stat);

            unsafe {
                CPUS_STAT_DOING.swap(false, Ordering::Relaxed);
            }
        });
    }
}

fn fetch_network_stat(detect_interval: Duration) {
    if !unsafe { NETWORK_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        NETWORK_STAT_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let network_stat = NetworkWithSpeed::get_networks_with_speed(detect_interval).unwrap();

            NETWORK_STAT.lock().unwrap().replace(network_stat);

            unsafe {
                NETWORK_STAT_DOING.swap(false, Ordering::Relaxed);
            }
        });
    }
}

fn fetch_volumes_stat(detect_interval: Duration) {
    if !unsafe { VOLUMES_STAT_DOING.compare_and_swap(false, true, Ordering::Relaxed) } {
        VOLUMES_STAT_LATEST_DETECT.lock().unwrap().replace(Instant::now());
        thread::spawn(move || {
            let volume_stat = VolumeWithSpeed::get_volumes_with_speed(detect_interval).unwrap();

            VOLUMES_STAT.lock().unwrap().replace(volume_stat);

            unsafe {
                VOLUMES_STAT_DOING.swap(false, Ordering::Relaxed);
            }
        });
    }
}

#[get("/hostname")]
fn hostname(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_string(
        hostname::get_hostname().unwrap(),
    )))
}

#[get("/hostname", rank = 2)]
fn hostname_401() -> Status {
    Status::Unauthorized
}

#[get("/kernel")]
fn kernel(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_string(
        kernel::get_kernel_version().unwrap(),
    )))
}

#[get("/kernel", rank = 2)]
fn kernel_401() -> Status {
    Status::Unauthorized
}

#[get("/uptime")]
fn uptime(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_u64(
        time::get_uptime().unwrap().as_secs(),
    )))
}

#[get("/uptime", rank = 2)]
fn uptime_401() -> Status {
    Status::Unauthorized
}

#[get("/time")]
fn time(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    let rtc_date_time = RTCDateTime::get_rtc_date_time().unwrap();

    let json_rtc_date_time = json!({
        "date": rtc_date_time.rtc_date,
        "time": rtc_date_time.rtc_time
    });

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json_rtc_date_time)))
}

#[get("/time", rank = 2)]
fn time_401() -> Status {
    Status::Unauthorized
}

#[get("/cpu")]
fn cpu(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    let cpus = CPU::get_cpus().unwrap();

    let load_average = LoadAverage::get_load_average().unwrap();

    let json_cpus = {
        let mut json_cpus = Vec::with_capacity(cpus.len());

        for cpu in cpus {
            json_cpus.push(json!({
                "model_name": cpu.model_name,
                "cores": cpu.cpu_cores,
                "threads": cpu.siblings,
                "mhz": cpu.cpus_mhz
            }));
        }

        json_cpus
    };

    let json_load_average = json!({
        "one": load_average.one,
        "five": load_average.five,
        "fifteen": load_average.fifteen
    });

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!({
        "load_average": json_load_average,
        "cpus": json_cpus
    }))))
}

#[get("/cpu", rank = 2)]
fn cpu_401() -> Status {
    Status::Unauthorized
}

#[get("/cpu-detect")]
fn cpu_detect(
    _auth: Auth,
    detect_interval: State<super::DetectInterval>,
) -> CacheResponse<JSONResponse<'static>> {
    fetch_cpus_stat(detect_interval.get_value());

    detect_cpus_stat_sleep(detect_interval.get_value(), true);

    let cpus_stat = CPUS_STAT.lock().unwrap();

    let cpus_stat: &[f64] = cpus_stat.as_ref().unwrap();

    let load_average = LoadAverage::get_load_average().unwrap();

    let cpus = CPU::get_cpus().unwrap();

    let json_cpus = {
        let mut json_cpus = Vec::with_capacity(cpus.len());

        for cpu in cpus {
            json_cpus.push(json!({
                "model_name": cpu.model_name,
                "cores": cpu.cpu_cores,
                "threads": cpu.siblings,
                "mhz": cpu.cpus_mhz
            }));
        }

        json_cpus
    };

    let json_load_average = json!({
        "one": load_average.one,
        "five": load_average.five,
        "fifteen": load_average.fifteen
    });

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!({
        "load_average": json_load_average,
        "cpus": json_cpus,
        "cpus_stat": cpus_stat
    }))))
}

#[get("/cpu-detect", rank = 2)]
fn cpu_detect_401() -> Status {
    Status::Unauthorized
}

#[get("/memory")]
fn memory(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    let free = Free::get_free().unwrap();

    let json_memory = json!({
        "total": free.mem.total,
        "used": free.mem.used,
        "free": free.mem.free,
        "shared": free.mem.shared,
        "buffers": free.mem.buffers,
        "cache": free.mem.cache,
        "available": free.mem.available
    });

    let json_swap = json!({
        "total": free.swap.total,
        "used": free.swap.used,
        "free": free.swap.free,
        "cache": free.swap.cache
    });

    let json_free = json!({
        "memory": json_memory,
        "swap": json_swap
    });

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!(json_free))))
}

#[get("/memory", rank = 2)]
fn memory_401() -> Status {
    Status::Unauthorized
}

#[get("/network-detect")]
fn network_detect(
    _auth: Auth,
    detect_interval: State<super::DetectInterval>,
) -> CacheResponse<JSONResponse<'static>> {
    fetch_network_stat(detect_interval.get_value());

    detect_network_stat_sleep(detect_interval.get_value(), true);

    let json_network = {
        let network_stat = NETWORK_STAT.lock().unwrap();

        let network_stat: &[NetworkWithSpeed] = network_stat.as_ref().unwrap();

        let mut json_network = Vec::with_capacity(network_stat.len());

        for network_with_speed in network_stat {
            json_network.push(json!({
                "interface": network_with_speed.network.interface,
                "upload_total": network_with_speed.network.transmit_bytes,
                "download_total": network_with_speed.network.receive_bytes,
                "upload_rate": network_with_speed.speed.transmit,
                "download_rate": network_with_speed.speed.receive
            }));
        }

        json_network
    };

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!(json_network))))
}

#[get("/network-detect", rank = 2)]
fn network_detect_401() -> Status {
    Status::Unauthorized
}

#[get("/volume")]
fn volume(_auth: Auth) -> CacheResponse<JSONResponse<'static>> {
    let json_volumes = {
        let volumes = Volume::get_volumes().unwrap();

        let mut json_volumes = Vec::with_capacity(volumes.len());

        for volume in volumes {
            json_volumes.push(json!({
                "device": volume.device,
                "size": volume.size,
                "used": volume.used,
                "read_total": volume.read_bytes,
                "write_total": volume.write_bytes,
                "mount_points": volume.points
            }));
        }

        json_volumes
    };

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!(json_volumes))))
}

#[get("/volume", rank = 2)]
fn volume_401() -> Status {
    Status::Unauthorized
}

#[get("/volume-detect")]
fn volume_detect(
    _auth: Auth,
    detect_interval: State<super::DetectInterval>,
) -> CacheResponse<JSONResponse<'static>> {
    fetch_volumes_stat(detect_interval.get_value());

    detect_volumes_stat_sleep(detect_interval.get_value(), true);

    let json_volumes = {
        let volumes_stat = VOLUMES_STAT.lock().unwrap();

        let volumes_stat: &[VolumeWithSpeed] = volumes_stat.as_ref().unwrap();

        let mut json_volumes = Vec::with_capacity(volumes_stat.len());

        for volume_with_speed in volumes_stat {
            json_volumes.push(json!({
                "device": volume_with_speed.volume.device,
                "size": volume_with_speed.volume.size,
                "used": volume_with_speed.volume.used,
                "read_total": volume_with_speed.volume.read_bytes,
                "write_total": volume_with_speed.volume.write_bytes,
                "read_rate": volume_with_speed.speed.read,
                "write_rate": volume_with_speed.speed.write,
                "mount_points": volume_with_speed.volume.points
            }));
        }

        json_volumes
    };

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!(json_volumes))))
}

#[get("/volume-detect", rank = 2)]
fn volume_detect_401() -> Status {
    Status::Unauthorized
}

#[get("/all")]
fn all(
    _auth: Auth,
    detect_interval: State<super::DetectInterval>,
) -> CacheResponse<JSONResponse<'static>> {
    fetch_cpus_stat(detect_interval.get_value());
    fetch_network_stat(detect_interval.get_value());
    fetch_volumes_stat(detect_interval.get_value());

    detect_all_sleep(detect_interval.get_value(), true);

    let cpus_stat = CPUS_STAT.lock().unwrap();

    let cpus_stat: &[f64] = cpus_stat.as_ref().unwrap();

    let load_average = LoadAverage::get_load_average().unwrap();

    let cpus = CPU::get_cpus().unwrap();

    let free = Free::get_free().unwrap();

    let hostname = hostname::get_hostname().unwrap();

    let kernel = kernel::get_kernel_version().unwrap();

    let uptime = time::get_uptime().unwrap();

    let rtc_date_time = RTCDateTime::get_rtc_date_time().unwrap();

    let json_cpus = {
        let mut json_cpus = Vec::with_capacity(cpus.len());

        for cpu in cpus {
            json_cpus.push(json!({
                "model_name": cpu.model_name,
                "cores": cpu.cpu_cores,
                "threads": cpu.siblings,
                "mhz": cpu.cpus_mhz
            }));
        }

        json_cpus
    };

    let json_load_average = json!({
        "one": load_average.one,
        "five": load_average.five,
        "fifteen": load_average.fifteen
    });

    let json_memory = json!({
        "total": free.mem.total,
        "used": free.mem.used,
        "free": free.mem.free,
        "shared": free.mem.shared,
        "buffers": free.mem.buffers,
        "cache": free.mem.cache,
        "available": free.mem.available
    });

    let json_swap = json!({
        "total": free.swap.total,
        "used": free.swap.used,
        "free": free.swap.free,
        "cache": free.swap.cache
    });

    let json_network = {
        let network_stat = NETWORK_STAT.lock().unwrap();

        let network_stat: &[NetworkWithSpeed] = network_stat.as_ref().unwrap();

        let mut json_network = Vec::with_capacity(network_stat.len());

        for network_with_speed in network_stat {
            json_network.push(json!({
                "interface": network_with_speed.network.interface,
                "upload_total": network_with_speed.network.transmit_bytes,
                "download_total": network_with_speed.network.receive_bytes,
                "upload_rate": network_with_speed.speed.transmit,
                "download_rate": network_with_speed.speed.receive
            }));
        }

        json_network
    };

    let json_volumes = {
        let volumes_stat = VOLUMES_STAT.lock().unwrap();

        let volumes_stat: &[VolumeWithSpeed] = volumes_stat.as_ref().unwrap();

        let mut json_volumes = Vec::with_capacity(volumes_stat.len());

        for volume_with_speed in volumes_stat {
            json_volumes.push(json!({
                "device": volume_with_speed.volume.device,
                "size": volume_with_speed.volume.size,
                "used": volume_with_speed.volume.used,
                "read_total": volume_with_speed.volume.read_bytes,
                "write_total": volume_with_speed.volume.write_bytes,
                "read_rate": volume_with_speed.speed.read,
                "write_rate": volume_with_speed.speed.write,
                "mount_points": volume_with_speed.volume.points
            }));
        }

        json_volumes
    };

    let json_rtc_date_time = json!({
        "date": rtc_date_time.rtc_date,
        "time": rtc_date_time.rtc_time
    });

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::from_json_value(json!({
        "hostname": hostname,
        "kernel": kernel,
        "uptime": uptime.as_secs(),
        "rtc_time": json_rtc_date_time,
        "load_average": json_load_average,
        "cpus": json_cpus,
        "cpus_stat": cpus_stat,
        "memory": json_memory,
        "swap": json_swap,
        "network": json_network,
        "volumes": json_volumes,
    }))))
}

#[get("/all", rank = 2)]
fn all_401() -> Status {
    Status::Unauthorized
}

#[get("/monitor")]
fn monitor(
    _auth: Auth,
    detect_interval: State<super::DetectInterval>,
) -> CacheResponse<JSONResponse<'static>> {
    fetch_cpus_stat(detect_interval.get_value());
    fetch_network_stat(detect_interval.get_value());
    fetch_volumes_stat(detect_interval.get_value());

    detect_all_sleep(detect_interval.get_value(), false);

    let load_average = LoadAverage::get_load_average().unwrap();

    let cpus = CPU::get_cpus().unwrap();

    let memory = Free::get_free().unwrap();

    let hostname = hostname::get_hostname().unwrap();

    let kernel = kernel::get_kernel_version().unwrap();

    let uptime = time::get_uptime().unwrap();

    let time = RTCDateTime::get_rtc_date_time().unwrap();

    let cpus_stat = CPUS_STAT.lock().unwrap();

    let cpus_stat: &[f64] = cpus_stat.as_ref().unwrap();

    let uptime_string = time::format_duration(uptime);

    let json_cpus = {
        let mut json_cpus = Vec::with_capacity(cpus.len());

        for cpu in cpus {
            let cpus_mhz_len = cpu.cpus_mhz.len();

            let mut mhz_list = LinkedList::new();

            let mut mhz_sum = 0f64;

            for mhz in cpu.cpus_mhz {
                mhz_sum += mhz;

                let adjusted_byte =
                    Byte::from_unit(mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                let mhz_string = format!(
                    "{:.2} {}Hz",
                    adjusted_byte.get_value(),
                    &adjusted_byte.get_unit().as_str()[..1]
                );

                mhz_list.push_back(json!({
                    "value": mhz,
                    "text": mhz_string
                }));
            }

            let mhz = mhz_sum / cpus_mhz_len as f64;
            let mhz_string = {
                let adjusted_byte =
                    Byte::from_unit(mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                format!(
                    "{:.2} {}Hz",
                    adjusted_byte.get_value(),
                    &adjusted_byte.get_unit().as_str()[..1]
                )
            };

            mhz_list.push_front(json!({
                "value": mhz,
                "text": mhz_string
            }));

            json_cpus.push(json!({
                "model_name": cpu.model_name,
                "cores": cpu.cpu_cores,
                "threads": cpu.siblings,
                "mhz": mhz_list
            }));
        }

        json_cpus
    };

    let memory_buff_and_cache = memory.mem.buffers + memory.mem.cache;

    let memory_total_string =
        Byte::from_bytes(memory.mem.total as u128).get_appropriate_unit(true).to_string();
    let memory_used_string =
        Byte::from_bytes(memory.mem.used as u128).get_appropriate_unit(true).to_string();
    let memory_buff_and_cache_string =
        Byte::from_bytes(memory_buff_and_cache as u128).get_appropriate_unit(true).to_string();

    let swap_total_string =
        Byte::from_bytes(memory.swap.total as u128).get_appropriate_unit(true).to_string();
    let swap_used_string =
        Byte::from_bytes(memory.swap.used as u128).get_appropriate_unit(true).to_string();
    let swap_cache_string =
        Byte::from_bytes(memory.swap.cache as u128).get_appropriate_unit(true).to_string();

    let json_network = {
        let network_stat = NETWORK_STAT.lock().unwrap();

        let network_stat: &[NetworkWithSpeed] = network_stat.as_ref().unwrap();

        let mut json_network = Vec::with_capacity(network_stat.len());

        for network_with_speed in network_stat {
            let upload_total_string =
                Byte::from_bytes(u128::from(network_with_speed.network.transmit_bytes))
                    .get_appropriate_unit(false)
                    .to_string();
            let download_total_string =
                Byte::from_bytes(u128::from(network_with_speed.network.receive_bytes))
                    .get_appropriate_unit(false)
                    .to_string();

            let upload_rate_string = {
                let mut s = Byte::from_unit(network_with_speed.speed.transmit, ByteUnit::B)
                    .unwrap()
                    .get_appropriate_unit(false)
                    .to_string();

                s.push_str("/s");

                s
            };

            let download_rate_string = {
                let mut s = Byte::from_unit(network_with_speed.speed.receive, ByteUnit::B)
                    .unwrap()
                    .get_appropriate_unit(false)
                    .to_string();

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

    let json_volumes = {
        let volumes_stat = VOLUMES_STAT.lock().unwrap();

        let volumes_stat: &[VolumeWithSpeed] = volumes_stat.as_ref().unwrap();

        let mut json_volumes = Vec::with_capacity(volumes_stat.len());

        for volume_with_speed in volumes_stat {
            let size_string = Byte::from_bytes(u128::from(volume_with_speed.volume.size))
                .get_appropriate_unit(false)
                .to_string();
            let used_string = Byte::from_bytes(u128::from(volume_with_speed.volume.used))
                .get_appropriate_unit(false)
                .to_string();

            let read_total_string =
                Byte::from_bytes(u128::from(volume_with_speed.volume.read_bytes))
                    .get_appropriate_unit(false)
                    .to_string();
            let write_total_string =
                Byte::from_bytes(u128::from(volume_with_speed.volume.write_bytes))
                    .get_appropriate_unit(false)
                    .to_string();

            let read_rate_string = {
                let mut s = Byte::from_bytes(volume_with_speed.speed.read as u128)
                    .get_appropriate_unit(false)
                    .to_string();

                s.push_str("/s");

                s
            };

            let download_rate_string = {
                let mut s = Byte::from_bytes(volume_with_speed.speed.write as u128)
                    .get_appropriate_unit(false)
                    .to_string();

                s.push_str("/s");

                s
            };

            json_volumes.push(json!({
                "device": volume_with_speed.volume.device,
                "size": {
                    "value": volume_with_speed.volume.size,
                    "text": size_string
                },
                "used": {
                    "value": volume_with_speed.volume.used,
                    "text": used_string
                },
                "read_total": {
                    "value": volume_with_speed.volume.read_bytes,
                    "text": read_total_string
                },
                "write_total": {
                    "value": volume_with_speed.volume.write_bytes,
                    "text": write_total_string
                },
                "read_rate": {
                    "value": volume_with_speed.speed.read,
                    "text": read_rate_string
                },
                "write_rate": {
                    "value": volume_with_speed.speed.write,
                    "text": download_rate_string
                },
                "mount_points": volume_with_speed.volume.points
            }));
        }

        json_volumes
    };

    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::JSONValue(json!({
        "hostname": hostname,
        "kernel": kernel,
        "uptime": {
            "value": uptime.as_secs(),
            "text": uptime_string
        },
        "rtc_time": format!("{} {}", time.rtc_date, time.rtc_time),
        "load_average": {
            "one": load_average.one,
            "five": load_average.five,
            "fifteen": load_average.fifteen
        },
        "cpus": json_cpus,
        "cpus_stat": cpus_stat,
        "memory": {
            "total": {
                "value": memory.mem.total,
                "text": memory_total_string
            },
            "used": {
                "value": memory.mem.used,
                "text": memory_used_string,
            },
            "buffer_cache": {
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
        "volumes": json_volumes,
    }))))
}

#[get("/monitor", rank = 2)]
fn monitor_401() -> Status {
    Status::Unauthorized
}

pub fn mounts(rocket: Rocket) -> Rocket {
    rocket
        .mount("/api", routes![hostname, hostname_401])
        .mount("/api", routes![kernel, kernel_401])
        .mount("/api", routes![uptime, uptime_401])
        .mount("/api", routes![time, time_401])
        .mount("/api", routes![cpu, cpu_401])
        .mount("/api", routes![cpu_detect, cpu_detect_401])
        .mount("/api", routes![memory, memory_401])
        .mount("/api", routes![network_detect, network_detect_401])
        .mount("/api", routes![volume, volume_401])
        .mount("/api", routes![volume_detect, volume_detect_401])
        .mount("/api", routes![all, all_401])
        .mount("/api", routes![monitor, monitor_401])
}

#[cfg(test)]
mod test {
    use super::*;

    use std::time::Duration;

    use rocket::http::Header;
    use rocket::local::Client;

    const TEST_DETECT_INTERVAL: u64 = 1000;
    const TEST_AUTH_KEY: &str = "magic";

    fn create_basic_rocket(has_auth_key: bool) -> Rocket {
        let rocket = rocket::ignite()
            .manage(super::super::DetectInterval(Duration::from_millis(TEST_DETECT_INTERVAL)));

        if has_auth_key {
            rocket.manage(super::super::AuthKey(Some(TEST_AUTH_KEY.to_string())))
        } else {
            rocket.manage(super::super::AuthKey(None))
        }
    }

    #[test]
    fn test_no_need_auth() {
        let rocket = create_basic_rocket(false).mount("/api", routes![hostname, hostname_401]);

        let client = Client::new(rocket).unwrap();

        {
            let mut req = client.get("/api/hostname");

            req.add_header(Header::new("Authorization", TEST_AUTH_KEY));

            let res = req.dispatch();

            assert_eq!(Status::Ok, res.status());
        }

        {
            let req = client.get("/api/hostname");

            let res = req.dispatch();

            assert_eq!(Status::Ok, res.status());
        }
    }

    #[test]
    fn test_need_auth() {
        let rocket = create_basic_rocket(true).mount("/api", routes![hostname, hostname_401]);

        let client = Client::new(rocket).unwrap();

        {
            let mut req = client.get("/api/hostname");

            req.add_header(Header::new("Authorization", TEST_AUTH_KEY));

            let res = req.dispatch();

            assert_eq!(Status::Ok, res.status());
        }

        {
            let req = client.get("/api/hostname");

            let res = req.dispatch();

            assert_eq!(Status::Unauthorized, res.status());
        }
    }
}
