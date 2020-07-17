#[macro_use]
extern crate concat_with;
extern crate clap;
extern crate terminal_size;

extern crate mprober_lib;
extern crate validators;

extern crate byte_unit;
extern crate chrono;
extern crate regex;
extern crate users;

extern crate getch;
extern crate termcolor;

#[macro_use]
extern crate lazy_static;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::io::{self, ErrorKind, Write};
use std::process as std_process;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mprober::*;

use clap::{App, Arg, ArgMatches, SubCommand};
use terminal_size::terminal_size;

use validators::prelude::*;

use byte_unit::{Byte, ByteUnit};
use chrono::SecondsFormat;
use regex::Regex;
use users::{Group, Groups, User, Users, UsersCache};

use getch::Getch;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

const APP_NAME: &str = "M Prober";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const DEFAULT_TERMINAL_WIDTH: usize = 64;
const MIN_TERMINAL_WIDTH: usize = 60;
const DEFAULT_INTERVAL: u64 = 333; // should be smaller than 1000 milliseconds

const CYAN_COLOR: Color = Color::Rgb(0, 177, 177);
const WHITE_COLOR: Color = Color::Rgb(219, 219, 219);
const RED_COLOR: Color = Color::Rgb(255, 95, 0);
const YELLOW_COLOR: Color = Color::Rgb(216, 177, 0);
const SKY_CYAN_COLOR: Color = Color::Rgb(107, 200, 200);

const DARK_CYAN_COLOR: Color = Color::Rgb(0, 95, 95);
const BLACK_COLOR: Color = Color::Rgb(28, 28, 28);
const WINE_COLOR: Color = Color::Rgb(215, 0, 0);
const ORANGE_COLOR: Color = Color::Rgb(215, 135, 0);
const DARK_BLUE_COLOR: Color = Color::Rgb(0, 0, 95);

const CLEAR_SCREEN_DATA: [u8; 11] =
    [0x1b, 0x5b, 0x33, 0x4a, 0x1b, 0x5b, 0x48, 0x1b, 0x5b, 0x32, 0x4a];

const ENV_LIGHT_MODE: &str = "MPROBER_LIGHT";
const ENV_FORCE_PLAIN: &str = "MPROBER_FORCE_PLAIN";

static mut LIGHT_MODE: bool = false;
static mut FORCE_PLAIN_MODE: bool = false;

lazy_static! {
    static ref COLOR_DEFAULT: ColorSpec = ColorSpec::new();
    static ref COLOR_LABEL: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(DARK_CYAN_COLOR));
            } else {
                color_spec.set_fg(Some(CYAN_COLOR));
            }
        }

        color_spec
    };
    static ref COLOR_NORMAL_TEXT: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(BLACK_COLOR));
            } else {
                color_spec.set_fg(Some(WHITE_COLOR));
            }
        }

        color_spec
    };
    static ref COLOR_BOLD_TEXT: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(BLACK_COLOR)).set_bold(true);
            } else {
                color_spec.set_fg(Some(WHITE_COLOR)).set_bold(true);
            }
        }

        color_spec
    };
    static ref COLOR_USED: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(WINE_COLOR));
            } else {
                color_spec.set_fg(Some(RED_COLOR));
            }
        }

        color_spec
    };
    static ref COLOR_CACHE: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(ORANGE_COLOR));
            } else {
                color_spec.set_fg(Some(YELLOW_COLOR));
            }
        }

        color_spec
    };
    static ref COLOR_BUFFERS: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe { FORCE_PLAIN_MODE } {
            if unsafe { LIGHT_MODE } {
                color_spec.set_fg(Some(DARK_BLUE_COLOR));
            } else {
                color_spec.set_fg(Some(SKY_CYAN_COLOR));
            }
        }

        color_spec
    };
}

macro_rules! set_color_mode {
    ($sub_matches:ident) => {
        unsafe {
            if $sub_matches.is_present("PLAIN") {
                FORCE_PLAIN_MODE = true;
            } else {
                match env::var_os(ENV_FORCE_PLAIN).map(|v| v.ne("0")) {
                    Some(b) => {
                        if b {
                            FORCE_PLAIN_MODE = true;
                        } else {
                            if $sub_matches.is_present("LIGHT") {
                                LIGHT_MODE = true;
                            } else {
                                LIGHT_MODE =
                                    env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);
                            }
                        }
                    }
                    None => {
                        if $sub_matches.is_present("LIGHT") {
                            LIGHT_MODE = true;
                        } else {
                            LIGHT_MODE =
                                env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);
                        }
                    }
                }
            }
        }
    };
}

macro_rules! monitor_handler {
    ($monitor:expr, $s:stmt) => {
        match $monitor {
            Some(monitor) => {
                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        if let b'q' = key {
                            break;
                        }
                    }

                    std_process::exit(0);
                });

                let sleep_interval = monitor;

                loop {
                    io::stdout().write_all(&CLEAR_SCREEN_DATA)?;

                    $s

                    thread::sleep(sleep_interval);
                }
            }
            None => {
                $s
            }
        }

        return Ok(());
    };
    ($monitor:expr, $monitor_interval_milli_secs:expr, $s:stmt) => {
        if $monitor {
            thread::spawn(move || {
                loop {
                    let key = Getch::new().getch().unwrap();

                    if let b'q' = key {
                        break;
                    }
                }

                std_process::exit(0);
            });

            let sleep_interval = Duration::from_millis($monitor_interval_milli_secs);

            loop {
                io::stdout().write_all(&CLEAR_SCREEN_DATA)?;

                $s

                thread::sleep(sleep_interval);
            }
        } else {
            $s
        }

        return Ok(());
    };
    ($monitor:expr, $s:stmt, $si:stmt, $no_self_sleep:expr) => {
        match $monitor {
            Some(monitor) => {
                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        if let b'q' = key {
                            break;
                        }
                    }

                    std_process::exit(0);
                });

                io::stdout().write_all(&CLEAR_SCREEN_DATA)?;

                $si

                let sleep_interval = monitor;

                loop {
                    if $no_self_sleep {
                        thread::sleep(sleep_interval);
                    }

                    io::stdout().write_all(&CLEAR_SCREEN_DATA)?;

                    $s
                }
            }
            None => {
                $s
            }
        }

        return Ok(());
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = get_matches();

    if matches.subcommand_matches("hostname").is_some() {
        handle_hostname()
    } else if matches.subcommand_matches("kernel").is_some() {
        handle_kernel()
    } else if let Some(sub_matches) = matches.subcommand_matches("uptime") {
        set_color_mode!(sub_matches);

        let monitor = sub_matches.is_present("MONITOR");
        let second = sub_matches.is_present("SECOND");

        handle_uptime(monitor, second)
    } else if let Some(sub_matches) = matches.subcommand_matches("time") {
        set_color_mode!(sub_matches);

        let monitor = sub_matches.is_present("MONITOR");

        handle_time(monitor)
    } else if let Some(sub_matches) = matches.subcommand_matches("cpu") {
        set_color_mode!(sub_matches);

        let monitor = get_monitor_duration(sub_matches)?;

        let separate = sub_matches.is_present("SEPARATE");
        let only_information = sub_matches.is_present("ONLY_INFORMATION");

        handle_cpu(monitor, separate, only_information)
    } else if let Some(sub_matches) = matches.subcommand_matches("memory") {
        set_color_mode!(sub_matches);

        let monitor = get_monitor_duration(sub_matches)?;
        let unit = get_byte_unit(sub_matches)?;

        handle_memory(monitor, unit)
    } else if let Some(sub_matches) = matches.subcommand_matches("network") {
        set_color_mode!(sub_matches);

        let monitor = get_monitor_duration(sub_matches)?;
        let unit = get_byte_unit(sub_matches)?;

        handle_network(monitor, unit)
    } else if let Some(sub_matches) = matches.subcommand_matches("volume") {
        set_color_mode!(sub_matches);

        let monitor = get_monitor_duration(sub_matches)?;
        let unit = get_byte_unit(sub_matches)?;

        let only_information = sub_matches.is_present("ONLY_INFORMATION");
        let mounts = sub_matches.is_present("MOUNTS");

        handle_volume(monitor, unit, only_information, mounts)
    } else if let Some(sub_matches) = matches.subcommand_matches("process") {
        set_color_mode!(sub_matches);

        let monitor = get_monitor_duration(sub_matches)?;
        let unit = get_byte_unit(sub_matches)?;

        let only_information = sub_matches.is_present("ONLY_INFORMATION");

        let top = match sub_matches.value_of("TOP") {
            Some(top) => Some(top.parse::<usize>()?),
            None => None,
        };

        let truncate = match sub_matches.value_of("TRUNCATE") {
            Some(truncate) => {
                let truncate = truncate.parse::<usize>()?;

                if truncate > 0 {
                    Some(truncate)
                } else {
                    None
                }
            }
            None => None,
        };

        let start_time = sub_matches.is_present("START_TIME");

        let user_filter = sub_matches.value_of("USER_FILTER");
        let group_filter = sub_matches.value_of("GROUP_FILTER");

        let program_filter = match sub_matches.value_of("PROGRAM_FILTER") {
            Some(program_filter) => Some(Regex::new(program_filter)?),
            None => None,
        };

        let tty_filter = match sub_matches.value_of("TTY_FILTER") {
            Some(tty_filter) => Some(Regex::new(tty_filter)?),
            None => None,
        };

        let pid_filter = match sub_matches.value_of("PID_FILTER") {
            Some(pid_filter) => Some(pid_filter.parse::<u32>()?),
            None => None,
        };

        handle_process(
            monitor,
            unit,
            only_information,
            top,
            truncate,
            start_time,
            user_filter,
            group_filter,
            program_filter,
            tty_filter,
            pid_filter,
        )
    } else if let Some(sub_matches) = matches.subcommand_matches("web") {
        let monitor = match sub_matches.value_of("MONITOR") {
            Some(monitor) => {
                let monitor = WebMonitorInterval::parse_str(monitor)
                    .map_err(|_| format!("`{}` is not a correct value for SECONDS", monitor))?;

                Duration::from_secs(monitor.get_number())
            }
            None => unreachable!(),
        };

        let address = sub_matches.value_of("ADDRESS").unwrap();

        let listen_port = match sub_matches.value_of("LISTEN_PORT") {
            Some(port) => {
                let port: u16 = port
                    .parse()
                    .map_err(|_| format!("`{}` is not a correct value for LISTEN_PORT", port))?;

                port
            }
            None => unreachable!(),
        };

        let auth_key = sub_matches.value_of("AUTH_KEY");

        let only_api = sub_matches.is_present("ONLY_API");

        handle_web(monitor, address, listen_port, auth_key, only_api)
    } else if let Some(sub_matches) = matches.subcommand_matches("benchmark") {
        let warming_up_duration = match sub_matches.value_of("WARMING_UP_DURATION") {
            Some(millisecond) => {
                let millisecond: u64 = millisecond.parse().map_err(|_| {
                    format!("`{}` is not a correct value for MILLI_SECONDS", millisecond)
                })?;

                Duration::from_millis(millisecond)
            }
            None => unreachable!(),
        };

        let benchmark_duration = match sub_matches.value_of("BENCHMARK_DURATION") {
            Some(millisecond) => {
                let millisecond: u64 = millisecond.parse().map_err(|_| {
                    format!("`{}` is not a correct value for MILLI_SECONDS", millisecond)
                })?;

                Duration::from_millis(millisecond)
            }
            None => unreachable!(),
        };

        let print_out = if sub_matches.is_present("VERBOSE") {
            benchmark::BenchmarkLog::Verbose
        } else {
            benchmark::BenchmarkLog::Normal
        };

        let disable_cpu = sub_matches.is_present("DISABLE_CPU");
        let enable_cpu = sub_matches.is_present("ENABLE_CPU");

        if enable_cpu && disable_cpu {
            return Err("Cannot determine whether to enable benchmarking CPU or not.".into());
        }

        let disable_memory = sub_matches.is_present("DISABLE_MEMORY");
        let enable_memory = sub_matches.is_present("ENABLE_MEMORY");

        if disable_memory && enable_memory {
            return Err("Cannot determine whether to enable benchmarking memory or not.".into());
        }

        let disable_volume = sub_matches.is_present("DISABLE_VOLUME");
        let enable_volume = sub_matches.is_present("ENABLE_VOLUME");

        if disable_volume && enable_volume {
            return Err("Cannot determine whether to enable benchmarking volumes or not.".into());
        }

        let default = !(enable_cpu || enable_memory || enable_volume);

        let cpu = if disable_cpu {
            false
        } else {
            default || enable_cpu
        };

        let memory = if disable_memory {
            false
        } else {
            default || enable_memory
        };

        let volume = if disable_volume {
            false
        } else {
            default || enable_volume
        };

        handle_benchmark(warming_up_duration, benchmark_duration, print_out, cpu, memory, volume)
    } else {
        Err("Please input a subcommand. Use `help` to see how to use this program.".into())
    }
}

#[inline]
fn handle_benchmark(
    warming_up_duration: Duration,
    benchmark_duration: Duration,
    print_out: benchmark::BenchmarkLog,
    cpu: bool,
    memory: bool,
    volume: bool,
) -> Result<(), Box<dyn Error>> {
    let benchmark_config = benchmark::BenchmarkConfig {
        warming_up_duration,
        benchmark_duration,
        print_out,
        cpu,
        memory,
        volume,
    };

    benchmark::run_benchmark(&benchmark_config)?;
    Ok(())
}

#[inline]
fn handle_web(
    monitor: Duration,
    address: &str,
    listen_port: u16,
    auth_key: Option<&str>,
    only_api: bool,
) -> Result<(), Box<dyn Error>> {
    rocket_mounts::launch(
        monitor,
        address.to_string(),
        listen_port,
        auth_key.map(|s| s.to_string()),
        only_api,
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_process(
    monitor: Option<Duration>,
    unit: Option<ByteUnit>,
    only_information: bool,
    top: Option<usize>,
    truncate: Option<usize>,
    start_time: bool,
    user_filter: Option<&str>,
    group_filter: Option<&str>,
    program_filter: Option<Regex>,
    tty_filter: Option<Regex>,
    pid_filter: Option<u32>,
) -> Result<(), Box<dyn Error>> {
    let user_filter = user_filter.as_deref();
    let group_filter = group_filter.as_deref();
    let program_filter = program_filter.as_ref();
    let tty_filter = tty_filter.as_ref();

    monitor_handler!(
        monitor,
        draw_process(
            top,
            truncate,
            unit,
            only_information,
            monitor,
            start_time,
            user_filter,
            group_filter,
            program_filter,
            tty_filter,
            pid_filter,
        )?,
        draw_process(
            top,
            truncate,
            unit,
            only_information,
            Some(Duration::from_millis(DEFAULT_INTERVAL)),
            start_time,
            user_filter,
            group_filter,
            program_filter,
            tty_filter,
            pid_filter,
        )?,
        only_information
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_process(
    mut top: Option<usize>,
    truncate: Option<usize>,
    unit: Option<ByteUnit>,
    only_information: bool,
    monitor: Option<Duration>,
    start_time: bool,
    user_filter: Option<&str>,
    group_filter: Option<&str>,
    program_filter: Option<&Regex>,
    tty_filter: Option<&Regex>,
    pid_filter: Option<u32>,
) -> Result<(), ScannerError> {
    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    let terminal_width = match terminal_size() {
        Some((width, height)) => {
            if monitor.is_some() {
                let height = (height.0 as usize).max(2) - 2;

                top = match top {
                    Some(top) => Some(top.min(height)),
                    None => Some(height),
                };
            }

            (width.0 as usize).max(MIN_TERMINAL_WIDTH)
        }
        None => DEFAULT_TERMINAL_WIDTH,
    };

    let user_cache = UsersCache::new();

    let uid_filter = match user_filter {
        Some(user_filter) => {
            match user_cache.get_user_by_name(user_filter) {
                Some(user) => Some(user.uid()),
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::InvalidInput,
                        format!("Cannot find the user `{}`.", user_filter),
                    )));
                }
            }
        }
        None => None,
    };

    let gid_filter = match group_filter {
        Some(group_filter) => {
            match user_cache.get_group_by_name(group_filter) {
                Some(group) => Some(group.gid()),
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::InvalidInput,
                        format!("Cannot find the group `{}`.", group_filter),
                    )));
                }
            }
        }
        None => None,
    };

    let process_filter = process::ProcessFilter {
        uid_filter,
        gid_filter,
        program_filter,
        tty_filter,
        pid_filter,
    };

    let (processes, percentage): (Vec<process::Process>, BTreeMap<u32, f64>) = if only_information {
        let mut processes_with_stats = process::get_processes_with_stat(&process_filter)?;

        processes_with_stats.sort_unstable_by(|(a, _), (b, _)| b.vsz.cmp(&a.vsz));

        if let Some(top) = top {
            if top < processes_with_stats.len() {
                unsafe {
                    processes_with_stats.set_len(top);
                }
            }
        }

        (processes_with_stats.into_iter().map(|(process, _)| process).collect(), BTreeMap::new())
    } else {
        let mut processes_with_percentage =
            process::get_processes_with_cpu_utilization_in_percentage(
                &process_filter,
                match monitor {
                    Some(monitor) => monitor,
                    None => Duration::from_millis(DEFAULT_INTERVAL),
                },
            )?;

        processes_with_percentage.sort_unstable_by(
            |(process_a, percentage_a), (process_b, percentage_b)| {
                let percentage_a = *percentage_a;
                let percentage_b = *percentage_b;

                if percentage_a > 0.01 {
                    if percentage_a > percentage_b {
                        Ordering::Less
                    } else if percentage_b > 0.01
                    // percentage_a == percentage_b hardly happens
                    {
                        Ordering::Greater
                    } else {
                        process_b.vsz.cmp(&process_a.vsz)
                    }
                } else if percentage_b > 0.01 {
                    if percentage_b > percentage_a {
                        Ordering::Greater
                    } else {
                        process_b.vsz.cmp(&process_a.vsz)
                    }
                } else {
                    process_b.vsz.cmp(&process_a.vsz)
                }
            },
        );

        if let Some(top) = top {
            if top < processes_with_percentage.len() {
                unsafe {
                    processes_with_percentage.set_len(top);
                }
            }
        }

        let mut processes = Vec::with_capacity(processes_with_percentage.len());
        let mut processes_percentage = BTreeMap::new();

        for (process, percentage) in processes_with_percentage {
            processes_percentage.insert(process.pid, percentage);

            processes.push(process);
        }

        (processes, processes_percentage)
    };

    let processes_len = processes.len();

    let mut pid: Vec<String> = Vec::with_capacity(processes_len);
    let mut ppid: Vec<String> = Vec::with_capacity(processes_len);
    let mut vsz: Vec<String> = Vec::with_capacity(processes_len);
    let mut rss: Vec<String> = Vec::with_capacity(processes_len);
    let mut anon: Vec<String> = Vec::with_capacity(processes_len);
    let mut thd: Vec<String> = Vec::with_capacity(processes_len);
    let mut tty: Vec<&str> = Vec::with_capacity(processes_len);
    let mut user: Vec<Arc<User>> = Vec::with_capacity(processes_len);
    let mut group: Vec<Arc<Group>> = Vec::with_capacity(processes_len);
    let mut program: Vec<&str> = Vec::with_capacity(processes_len);
    let mut state: Vec<&'static str> = Vec::with_capacity(processes_len);

    for process in processes.iter() {
        pid.push(process.pid.to_string());
        ppid.push(process.ppid.to_string());

        let (p_vsz, p_rss, p_anon) = (
            Byte::from_bytes(process.vsz as u128),
            Byte::from_bytes(process.rss as u128),
            Byte::from_bytes(process.rss_anon as u128),
        );

        match unit {
            Some(byte_unit) => {
                vsz.push(p_vsz.get_adjusted_unit(byte_unit).format(1));
                rss.push(p_rss.get_adjusted_unit(byte_unit).format(1));
                anon.push(p_anon.get_adjusted_unit(byte_unit).format(1));
            }
            None => {
                vsz.push(p_vsz.get_appropriate_unit(true).format(1));
                rss.push(p_rss.get_appropriate_unit(true).format(1));
                anon.push(p_anon.get_appropriate_unit(true).format(1));
            }
        }

        tty.push(process.tty.as_deref().unwrap_or(""));

        thd.push(process.threads.to_string());

        // TODO: musl cannot directly handle dynamic users (with systemd). It causes `UserCache` returns `None`.
        user.push(
            user_cache
                .get_user_by_uid(process.effective_uid)
                .unwrap_or_else(|| Arc::new(User::new(0, "systemd?", 0))),
        );
        group.push(
            user_cache
                .get_group_by_gid(process.effective_gid)
                .unwrap_or_else(|| Arc::new(Group::new(0, "systemd?"))),
        );

        program.push(process.program.as_str());
        state.push(process.state.as_str());
    }

    let truncate_inc = truncate.map(|t| t + 1).unwrap_or(usize::max_value());

    let pid_len = pid.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let ppid_len = ppid.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let vsz_len = vsz.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let rss_len = rss.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let anon_len = anon.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let thd_len = thd.iter().map(|s| s.len()).max().map(|s| s.max(3)).unwrap_or(0);
    let tty_len = tty.iter().map(|s| s.len()).max().map(|s| s.max(4)).unwrap_or(0);
    let user_len = user
        .iter()
        .map(|user| user.name().len())
        .max()
        .map(|s| s.min(truncate_inc).max(4))
        .unwrap_or(truncate_inc);
    let group_len = group
        .iter()
        .map(|group| group.name().len())
        .max()
        .map(|s| s.min(truncate_inc).max(5))
        .unwrap_or(truncate_inc);
    let program_len = program
        .iter()
        .map(|s| s.len())
        .max()
        .map(|s| s.min(truncate_inc).max(7))
        .unwrap_or(truncate_inc);
    let state_len = state.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);

    #[allow(clippy::never_loop)]
    loop {
        let mut width = 0;

        stdout.set_color(&*COLOR_LABEL)?;

        if width + pid_len > terminal_width {
            break;
        }

        for _ in 3..pid_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "PID")?; // 3
        width += 3;

        if width + 1 + ppid_len > terminal_width {
            break;
        }

        for _ in 3..ppid_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "PPID")?; // 4
        width += 4;

        if width + 5 > terminal_width {
            break;
        }

        write!(&mut stdout, "   PR")?; // 5
        width += 5;

        if width + 4 > terminal_width {
            break;
        }

        write!(&mut stdout, "  NI")?; // 4
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                break;
            }

            write!(&mut stdout, " %CPU")?; // 5
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            break;
        }

        for _ in 2..vsz_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "VSZ")?; // 3
        width += 3;

        if width + 1 + rss_len > terminal_width {
            break;
        }

        for _ in 2..rss_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "RSS")?; // 3
        width += 3;

        if width + 1 + anon_len > terminal_width {
            break;
        }

        for _ in 3..anon_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "ANON")?; // 4
        width += 4;

        if width + 1 + thd_len > terminal_width {
            break;
        }

        for _ in 2..thd_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        write!(&mut stdout, "THD")?; // 3
        width += 3;

        if width + 1 + tty_len > terminal_width {
            break;
        }

        write!(&mut stdout, " TTY")?; // 4
        width += 4;

        for _ in 3..tty_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if width + 1 + user_len > terminal_width {
            break;
        }

        write!(&mut stdout, " USER")?; // 5
        width += 5;

        for _ in 4..user_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if width + 1 + group_len > terminal_width {
            break;
        }

        write!(&mut stdout, " GROUP")?; // 6
        width += 6;

        for _ in 5..group_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if width + 1 + program_len > terminal_width {
            break;
        }

        write!(&mut stdout, " PROGRAM")?; // 8
        width += 8;

        for _ in 7..program_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if width + 1 + state_len > terminal_width {
            break;
        }

        write!(&mut stdout, " STATE")?; // 6
        width += 6;

        for _ in 5..state_len {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if start_time {
            if width + 21 > terminal_width {
                break;
            }

            write!(&mut stdout, " START")?; // 6
            width += 6;

            for _ in 5..20 {
                write!(&mut stdout, " ")?; // 1
                width += 1;
            }
        }

        if width + 8 > terminal_width {
            break;
        }

        write!(&mut stdout, " COMMAND")?; // 8

        break;
    }

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout)?;

    let mut pid_iter = pid.into_iter();
    let mut ppid_iter = ppid.into_iter();
    let mut vsz_iter = vsz.into_iter();
    let mut rss_iter = rss.into_iter();
    let mut tty_iter = tty.into_iter();
    let mut anon_iter = anon.into_iter();
    let mut thd_iter = thd.into_iter();
    let mut user_iter = user.into_iter();
    let mut group_iter = group.into_iter();
    let mut program_iter = program.into_iter();
    let mut state_iter = state.into_iter();

    for process in processes.iter() {
        let mut width = 0;

        if width + pid_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let pid = pid_iter.next().unwrap();

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        write!(&mut stdout, "{1:>0$}", pid_len, pid)?;
        width += pid_len;

        if width + 1 + ppid_len > terminal_width {
            continue;
        }

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;

        let ppid = ppid_iter.next().unwrap();

        for _ in 0..=(ppid_len - ppid.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        stdout.write_all(ppid.as_bytes())?;
        width += ppid.len();

        if width + 5 > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        if let Some(real_time_priority) = process.real_time_priority {
            write!(&mut stdout, "{:>5}", format!("*{}", real_time_priority))?;
        } else {
            write!(&mut stdout, "{:>5}", process.priority)?;
        }
        width += 5;

        if width + 4 > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        write!(&mut stdout, "{:>4}", process.nice)?;
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                stdout.set_color(&*COLOR_DEFAULT)?;
                writeln!(&mut stdout)?;

                continue;
            }

            write!(&mut stdout, " {:>4.1}", percentage.get(&process.pid).unwrap() * 100.0)?;
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let vsz = vsz_iter.next().unwrap();

        for _ in 0..=(vsz_len - vsz.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        stdout.write_all(vsz.as_bytes())?;
        width += vsz.len();

        if width + 1 + rss_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let rss = rss_iter.next().unwrap();

        for _ in 0..=(rss_len - rss.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        stdout.write_all(rss.as_bytes())?;
        width += rss.len();

        if width + 1 + anon_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let anon = anon_iter.next().unwrap();

        for _ in 0..=(anon_len - anon.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        stdout.write_all(anon.as_bytes())?;
        width += anon.len();

        if width + 1 + thd_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let thd = thd_iter.next().unwrap();

        for _ in 0..=(thd_len - thd.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        stdout.write_all(thd.as_bytes())?;
        width += thd.len();

        if width + 1 + tty_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let tty = tty_iter.next().unwrap();

        write!(&mut stdout, " ")?; // 1
        width += 1;

        stdout.write_all(tty.as_bytes())?;
        width += tty.len();

        for _ in 0..(tty_len - tty.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if width + 1 + user_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let user = user_iter.next().unwrap();

        write!(&mut stdout, " ")?; // 1
        width += 1;

        {
            let s = user.name().to_str().unwrap();

            if s.len() > truncate_inc {
                stdout.write_all(s[..(truncate_inc - 1)].as_bytes())?;
                write!(&mut stdout, "+")?; // 1
                width += truncate_inc;

                for _ in truncate_inc..4 {
                    write!(&mut stdout, " ")?; // 1
                    width += 1;
                }
            } else {
                stdout.write_all(s.as_bytes())?;
                width += s.len();

                for _ in 0..(user_len - s.len()) {
                    write!(&mut stdout, " ")?; // 1
                    width += 1;
                }
            }
        }

        if width + 1 + group_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let group = group_iter.next().unwrap();

        write!(&mut stdout, " ")?; // 1
        width += 1;

        {
            let s = group.name().to_str().unwrap();

            if s.len() > truncate_inc {
                stdout.write_all(s[..(truncate_inc - 1)].as_bytes())?;
                write!(&mut stdout, "+")?; // 1
                width += truncate_inc;

                for _ in truncate_inc..5 {
                    write!(&mut stdout, " ")?; // 1
                    width += 1;
                }
            } else {
                stdout.write_all(s.as_bytes())?;
                width += s.len();

                for _ in 0..(group_len - s.len()) {
                    write!(&mut stdout, " ")?; // 1
                    width += 1;
                }
            }
        }

        if width + 1 + program_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let program = program_iter.next().unwrap();

        write!(&mut stdout, " ")?; // 1
        width += 1;

        if program.len() > truncate_inc {
            stdout.write_all(program[..(truncate_inc - 1)].as_bytes())?;
            write!(&mut stdout, "+")?; // 1
            width += truncate_inc;

            for _ in truncate_inc..7 {
                write!(&mut stdout, " ")?; // 1
                width += 1;
            }
        } else {
            stdout.write_all(program.as_bytes())?;
            width += program.len();

            for _ in 0..(program_len - program.len()) {
                write!(&mut stdout, " ")?; // 1
                width += 1;
            }
        }

        if width + 1 + state_len > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        let state = state_iter.next().unwrap();

        write!(&mut stdout, " ")?; // 1
        width += 1;

        stdout.write_all(state.as_bytes())?;
        width += state.len();

        for _ in 0..(state_len - state.len()) {
            write!(&mut stdout, " ")?; // 1
            width += 1;
        }

        if start_time {
            if width + 21 > terminal_width {
                stdout.set_color(&*COLOR_DEFAULT)?;
                writeln!(&mut stdout)?;

                continue;
            }

            write!(&mut stdout, " ")?; // 1

            stdout.write_all(
                process.start_time.to_rfc3339_opts(SecondsFormat::Secs, true).as_bytes(),
            )?;

            width += 21;
        }

        if width + 8 > terminal_width {
            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            continue;
        }

        write!(&mut stdout, " ")?; // 1
        width += 1;

        let remain_width = terminal_width - width;

        if process.cmdline.len() > remain_width {
            let cmdline =
                String::from_utf8_lossy(&process.cmdline.as_bytes()[..(remain_width - 1)]);

            stdout.write_all(cmdline.as_bytes())?;
            write!(&mut stdout, "+")?; // 1
        } else {
            stdout.write_all(process.cmdline.as_bytes())?;
        }

        stdout.set_color(&*COLOR_DEFAULT)?;
        writeln!(&mut stdout)?;
    }

    output.print(&stdout)?;

    Ok(())
}

fn handle_volume(
    monitor: Option<Duration>,
    unit: Option<ByteUnit>,
    only_information: bool,
    mounts: bool,
) -> Result<(), Box<dyn Error>> {
    monitor_handler!(
        monitor,
        draw_volume(unit, only_information, mounts, monitor)?,
        draw_volume(unit, only_information, mounts, None)?,
        only_information
    );
}

fn draw_volume(
    unit: Option<ByteUnit>,
    only_information: bool,
    mounts: bool,
    monitor: Option<Duration>,
) -> Result<(), ScannerError> {
    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    let terminal_width = terminal_size()
        .map(|(width, _)| (width.0 as usize).max(MIN_TERMINAL_WIDTH))
        .unwrap_or(DEFAULT_TERMINAL_WIDTH);

    if only_information {
        let volumes = volume::get_volumes()?;

        let volumes_len = volumes.len();

        debug_assert!(volumes_len > 0);

        let mut volumes_size: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used_percentage: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_read_total: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_write_total: Vec<String> = Vec::with_capacity(volumes_len);

        for volume in volumes.iter() {
            let size = Byte::from_bytes(u128::from(volume.size));

            let used = Byte::from_bytes(u128::from(volume.used));

            let used_percentage =
                format!("{:.2}%", (volume.used * 100) as f64 / volume.size as f64);

            let read_total = Byte::from_bytes(u128::from(volume.stat.read_bytes));

            let write_total = Byte::from_bytes(u128::from(volume.stat.write_bytes));

            let (size, used, read_total, write_total) = match unit {
                Some(unit) => {
                    (
                        size.get_adjusted_unit(unit).to_string(),
                        used.get_adjusted_unit(unit).to_string(),
                        read_total.get_adjusted_unit(unit).to_string(),
                        write_total.get_adjusted_unit(unit).to_string(),
                    )
                }
                None => {
                    (
                        size.get_appropriate_unit(false).to_string(),
                        used.get_appropriate_unit(false).to_string(),
                        read_total.get_appropriate_unit(false).to_string(),
                        write_total.get_appropriate_unit(false).to_string(),
                    )
                }
            };

            volumes_size.push(size);
            volumes_used.push(used);
            volumes_used_percentage.push(used_percentage);
            volumes_read_total.push(read_total);
            volumes_write_total.push(write_total);
        }

        let devices_len = volumes.iter().map(|volume| volume.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let volumes_size_len = volumes_size.iter().map(|size| size.len()).max().unwrap();
        let volumes_used_len = volumes_used.iter().map(|used| used.len()).max().unwrap();
        let volumes_used_percentage_len = volumes_used_percentage
            .iter()
            .map(|used_percentage| used_percentage.len())
            .max()
            .unwrap();
        let volumes_read_total_len =
            volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_total_len =
            volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width
            - devices_len
            - 4
            - volumes_used_len
            - 3
            - volumes_size_len
            - 2
            - volumes_used_percentage_len
            - 1;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", devices_len_inc + volumes_read_total_len, "Read Data")?;

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", volumes_write_total_len, "Written Data")?;

        writeln!(&mut stdout)?;

        let mut volumes_size_iter = volumes_size.into_iter();
        let mut volumes_used_iter = volumes_used.into_iter();
        let mut volumes_used_percentage_iter = volumes_used_percentage.into_iter();
        let mut volumes_read_total_iter = volumes_read_total.into_iter();
        let mut volumes_write_total_iter = volumes_write_total.into_iter();

        for volume in volumes.into_iter() {
            let size = volumes_size_iter.next().unwrap();

            let used = volumes_used_iter.next().unwrap();

            let used_percentage = volumes_used_percentage_iter.next().unwrap();

            let read_total = volumes_read_total_iter.next().unwrap();

            let write_total = volumes_write_total_iter.next().unwrap();

            stdout.set_color(&*COLOR_LABEL)?;
            write!(&mut stdout, "{1:<0$}", devices_len_inc, volume.device)?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            for _ in 0..(volumes_read_total_len - read_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read_total.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(volumes_write_total_len - write_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write_total.as_bytes())?;

            writeln!(&mut stdout)?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;

            for _ in 0..devices_len {
                write!(&mut stdout, " ")?;
            }

            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64 / volume.size as f64;

            let progress_used = (volume.used as f64 * f).floor() as usize;

            stdout.set_color(&*COLOR_USED)?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, "] ")?; // 2

            for _ in 0..(volumes_used_len - used.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            stdout.write_all(used.as_bytes())?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, " / ")?; // 3

            for _ in 0..(volumes_size_len - size.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            stdout.write_all(size.as_bytes())?;

            write!(&mut stdout, " (")?; // 2

            for _ in 0..(volumes_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.write_all(used_percentage.as_bytes())?;

            write!(&mut stdout, ")")?; // 1

            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            if mounts {
                stdout.set_color(&*COLOR_NORMAL_TEXT)?;

                for point in volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout)?;
                }
            }
        }
    } else {
        let volumes_with_speed = volume::get_volumes_with_speed(match monitor {
            Some(monitor) => monitor,
            None => Duration::from_millis(DEFAULT_INTERVAL),
        })?;

        let volumes_with_speed_len = volumes_with_speed.len();

        debug_assert!(volumes_with_speed_len > 0);

        let mut volumes_size: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_used: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_used_percentage: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_read: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_read_total: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_write: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_write_total: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        for (volume, volume_speed) in volumes_with_speed.iter() {
            let size = Byte::from_bytes(u128::from(volume.size));

            let used = Byte::from_bytes(u128::from(volume.used));

            let used_percentage =
                format!("{:.2}%", (volume.used * 100) as f64 / volume.size as f64);

            let read = Byte::from_unit(volume_speed.read, ByteUnit::B).unwrap();
            let read_total = Byte::from_bytes(u128::from(volume.stat.read_bytes));

            let write = Byte::from_unit(volume_speed.write, ByteUnit::B).unwrap();
            let write_total = Byte::from_bytes(u128::from(volume.stat.write_bytes));

            let (size, used, mut read, read_total, mut write, write_total) = match unit {
                Some(unit) => {
                    (
                        size.get_adjusted_unit(unit).to_string(),
                        used.get_adjusted_unit(unit).to_string(),
                        read.get_adjusted_unit(unit).to_string(),
                        read_total.get_adjusted_unit(unit).to_string(),
                        write.get_adjusted_unit(unit).to_string(),
                        write_total.get_adjusted_unit(unit).to_string(),
                    )
                }
                None => {
                    (
                        size.get_appropriate_unit(false).to_string(),
                        used.get_appropriate_unit(false).to_string(),
                        read.get_appropriate_unit(false).to_string(),
                        read_total.get_appropriate_unit(false).to_string(),
                        write.get_appropriate_unit(false).to_string(),
                        write_total.get_appropriate_unit(false).to_string(),
                    )
                }
            };

            read.push_str("/s");
            write.push_str("/s");

            volumes_size.push(size);
            volumes_used.push(used);
            volumes_used_percentage.push(used_percentage);
            volumes_read.push(read);
            volumes_read_total.push(read_total);
            volumes_write.push(write);
            volumes_write_total.push(write_total);
        }

        let devices_len =
            volumes_with_speed.iter().map(|(volume, _)| volume.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let volumes_size_len = volumes_size.iter().map(|size| size.len()).max().unwrap();
        let volumes_used_len = volumes_used.iter().map(|used| used.len()).max().unwrap();
        let volumes_used_percentage_len = volumes_used_percentage
            .iter()
            .map(|used_percentage| used_percentage.len())
            .max()
            .unwrap();
        let volumes_read_len = volumes_read.iter().map(|read| read.len()).max().unwrap().max(12);
        let volumes_read_total_len =
            volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_len =
            volumes_write.iter().map(|write| write.len()).max().unwrap().max(12);
        let volumes_write_total_len =
            volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width
            - devices_len
            - 4
            - volumes_used_len
            - 3
            - volumes_size_len
            - 2
            - volumes_used_percentage_len
            - 1;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", devices_len_inc + volumes_read_len, "Reading Rate")?;

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", volumes_read_total_len, "Read Data")?;

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", volumes_write_len, "Writing Rate")?;

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", volumes_write_total_len, "Written Data")?;

        writeln!(&mut stdout)?;

        let mut volumes_size_iter = volumes_size.into_iter();
        let mut volumes_used_iter = volumes_used.into_iter();
        let mut volumes_used_percentage_iter = volumes_used_percentage.into_iter();
        let mut volumes_read_iter = volumes_read.into_iter();
        let mut volumes_read_total_iter = volumes_read_total.into_iter();
        let mut volumes_write_iter = volumes_write.into_iter();
        let mut volumes_write_total_iter = volumes_write_total.into_iter();

        for (volume, _) in volumes_with_speed.into_iter() {
            let size = volumes_size_iter.next().unwrap();

            let used = volumes_used_iter.next().unwrap();

            let used_percentage = volumes_used_percentage_iter.next().unwrap();

            let read = volumes_read_iter.next().unwrap();
            let read_total = volumes_read_total_iter.next().unwrap();

            let write = volumes_write_iter.next().unwrap();
            let write_total = volumes_write_total_iter.next().unwrap();

            stdout.set_color(&*COLOR_LABEL)?;
            write!(&mut stdout, "{1:<0$}", devices_len_inc, volume.device)?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            for _ in 0..(volumes_read_len - read.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(volumes_read_total_len - read_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read_total.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(volumes_write_len - write.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(volumes_write_total_len - write_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write_total.as_bytes())?;

            writeln!(&mut stdout)?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;

            for _ in 0..devices_len {
                write!(&mut stdout, " ")?;
            }

            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64 / volume.size as f64;

            let progress_used = (volume.used as f64 * f).floor() as usize;

            stdout.set_color(&*COLOR_USED)?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, "] ")?; // 2

            for _ in 0..(volumes_used_len - used.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            stdout.write_all(used.as_bytes())?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, " / ")?; // 3

            for _ in 0..(volumes_size_len - size.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            stdout.write_all(size.as_bytes())?;

            write!(&mut stdout, " (")?; // 2

            for _ in 0..(volumes_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.write_all(used_percentage.as_bytes())?;

            write!(&mut stdout, ")")?; // 1

            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;

            if mounts {
                stdout.set_color(&*COLOR_NORMAL_TEXT)?;

                for point in volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout)?;
                }
            }
        }
    }

    output.print(&stdout)?;

    Ok(())
}

fn handle_network(monitor: Option<Duration>, unit: Option<ByteUnit>) -> Result<(), Box<dyn Error>> {
    monitor_handler!(monitor, draw_network(unit, monitor)?, draw_network(unit, None)?, false);
}

fn draw_network(unit: Option<ByteUnit>, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let networks_with_speed = network::get_networks_with_speed(match monitor {
        Some(monitor) => monitor,
        None => Duration::from_millis(DEFAULT_INTERVAL),
    })?;

    let networks_with_speed_len = networks_with_speed.len();

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    debug_assert!(networks_with_speed_len > 0);

    let mut uploads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut uploads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    let mut downloads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut downloads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    for (network, network_speed) in networks_with_speed.iter() {
        let upload = Byte::from_unit(network_speed.transmit, ByteUnit::B).unwrap();
        let upload_total = Byte::from_bytes(u128::from(network.stat.transmit_bytes));

        let download = Byte::from_unit(network_speed.receive, ByteUnit::B).unwrap();
        let download_total = Byte::from_bytes(u128::from(network.stat.receive_bytes));

        let (mut upload, upload_total, mut download, download_total) = match unit {
            Some(unit) => {
                (
                    upload.get_adjusted_unit(unit).to_string(),
                    upload_total.get_adjusted_unit(unit).to_string(),
                    download.get_adjusted_unit(unit).to_string(),
                    download_total.get_adjusted_unit(unit).to_string(),
                )
            }
            None => {
                (
                    upload.get_appropriate_unit(false).to_string(),
                    upload_total.get_appropriate_unit(false).to_string(),
                    download.get_appropriate_unit(false).to_string(),
                    download_total.get_appropriate_unit(false).to_string(),
                )
            }
        };

        upload.push_str("/s");
        download.push_str("/s");

        uploads.push(upload);
        uploads_total.push(upload_total);
        downloads.push(download);
        downloads_total.push(download_total);
    }

    let interface_len =
        networks_with_speed.iter().map(|(network, _)| network.interface.len()).max().unwrap();
    let interface_len_inc = interface_len + 1;

    let upload_len = uploads.iter().map(|upload| upload.len()).max().unwrap().max(11);
    let upload_total_len =
        uploads_total.iter().map(|upload_total| upload_total.len()).max().unwrap().max(13);
    let download_len = downloads.iter().map(|download| download.len()).max().unwrap().max(13);
    let download_total_len =
        downloads_total.iter().map(|download_total| download_total.len()).max().unwrap().max(15);

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "{1:>0$}", interface_len_inc + upload_len, "Upload Rate")?;

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "{1:>0$}", upload_total_len, "Uploaded Data")?;

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "{1:>0$}", download_len, "Download Rate")?;

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "{1:>0$}", download_total_len, "Downloaded Data")?;

    writeln!(&mut stdout)?;

    let mut uploads_iter = uploads.into_iter();
    let mut uploads_total_iter = uploads_total.into_iter();
    let mut downloads_iter = downloads.into_iter();
    let mut downloads_total_iter = downloads_total.into_iter();

    for (network, _) in networks_with_speed.into_iter() {
        let upload = uploads_iter.next().unwrap();
        let upload_total = uploads_total_iter.next().unwrap();

        let download = downloads_iter.next().unwrap();
        let download_total = downloads_total_iter.next().unwrap();

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:<0$}", interface_len_inc, network.interface)?;

        stdout.set_color(&*COLOR_BOLD_TEXT)?;

        for _ in 0..(upload_len - upload.len()) {
            write!(&mut stdout, " ")?;
        }

        stdout.write_all(upload.as_bytes())?;

        write!(&mut stdout, "   ")?;

        for _ in 0..(upload_total_len - upload_total.len()) {
            write!(&mut stdout, " ")?;
        }

        stdout.write_all(upload_total.as_bytes())?;

        write!(&mut stdout, "   ")?;

        for _ in 0..(download_len - download.len()) {
            write!(&mut stdout, " ")?;
        }

        stdout.write_all(download.as_bytes())?;

        write!(&mut stdout, "   ")?;

        for _ in 0..(download_total_len - download_total.len()) {
            write!(&mut stdout, " ")?;
        }

        stdout.write_all(download_total.as_bytes())?;

        stdout.set_color(&*COLOR_DEFAULT)?;
        writeln!(&mut stdout)?;
    }

    output.print(&stdout)?;

    Ok(())
}

fn handle_memory(monitor: Option<Duration>, unit: Option<ByteUnit>) -> Result<(), Box<dyn Error>> {
    monitor_handler!(monitor, draw_memory(unit)?);
}

fn draw_memory(unit: Option<ByteUnit>) -> Result<(), ScannerError> {
    let free = memory::free()?;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    let (mem_used, mem_total, swap_used, swap_total) = {
        let (mem_used, mem_total, swap_used, swap_total) = (
            Byte::from_bytes(free.mem.used as u128),
            Byte::from_bytes(free.mem.total as u128),
            Byte::from_bytes(free.swap.used as u128),
            Byte::from_bytes(free.swap.total as u128),
        );

        match unit {
            Some(unit) => {
                (
                    mem_used.get_adjusted_unit(unit).to_string(),
                    mem_total.get_adjusted_unit(unit).to_string(),
                    swap_used.get_adjusted_unit(unit).to_string(),
                    swap_total.get_adjusted_unit(unit).to_string(),
                )
            }
            None => {
                (
                    mem_used.get_appropriate_unit(true).to_string(),
                    mem_total.get_appropriate_unit(true).to_string(),
                    swap_used.get_appropriate_unit(true).to_string(),
                    swap_total.get_appropriate_unit(true).to_string(),
                )
            }
        }
    };

    let used_len = mem_used.len().max(swap_used.len());
    let total_len = mem_total.len().max(swap_total.len());

    let mem_percentage = format!("{:.2}%", free.mem.used as f64 * 100f64 / free.mem.total as f64);
    let swap_percentage =
        format!("{:.2}%", free.swap.used as f64 * 100f64 / free.swap.total as f64);

    let percentage_len = mem_percentage.len().max(swap_percentage.len());

    let terminal_width = terminal_size()
        .map(|(width, _)| (width.0 as usize).max(MIN_TERMINAL_WIDTH))
        .unwrap_or(DEFAULT_TERMINAL_WIDTH);

    // Memory

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "Memory")?; // 6

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " [")?; // 2

    let progress_max = terminal_width - 10 - used_len - 3 - total_len - 2 - percentage_len - 1;

    let f = progress_max as f64 / free.mem.total as f64;

    let progress_used = (free.mem.used as f64 * f).floor() as usize;

    stdout.set_color(&*COLOR_USED)?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.mem.cache as f64 * f).floor() as usize;

    stdout.set_color(&*COLOR_CACHE)?;
    for _ in 0..progress_cache {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "$")?; // 1
        } else {
            write!(&mut stdout, "|")?; // 1
        }
    }

    let progress_buffers = (free.mem.buffers as f64 * f).floor() as usize;

    stdout.set_color(&*COLOR_BUFFERS)?;
    for _ in 0..progress_buffers {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "#")?; // 1
        } else {
            write!(&mut stdout, "|")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache - progress_buffers) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - mem_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(mem_used.as_bytes())?;

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - mem_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(mem_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - mem_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(mem_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    writeln!(&mut stdout)?;

    // Swap

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "Swap  ")?; // 6

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " [")?; // 2

    let f = progress_max as f64 / free.swap.total as f64;

    let progress_used = (free.swap.used as f64 * f).floor() as usize;

    stdout.set_color(&*COLOR_USED)?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.swap.cache as f64 * f).floor() as usize;

    stdout.set_color(&*COLOR_CACHE)?;
    for _ in 0..progress_cache {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "$")?; // 1
        } else {
            write!(&mut stdout, "|")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - swap_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(swap_used.as_bytes())?;

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - swap_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(swap_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - swap_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(swap_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout)?;

    output.print(&stdout)?;

    Ok(())
}

fn handle_cpu(
    monitor: Option<Duration>,
    separate: bool,
    only_information: bool,
) -> Result<(), Box<dyn Error>> {
    monitor_handler!(
        monitor,
        draw_cpu_info(separate, only_information, monitor)?,
        draw_cpu_info(separate, only_information, None)?,
        only_information
    );
}

fn draw_cpu_info(
    separate: bool,
    only_information: bool,
    monitor: Option<Duration>,
) -> Result<(), ScannerError> {
    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    let terminal_width = terminal_size()
        .map(|(width, _)| (width.0 as usize).max(MIN_TERMINAL_WIDTH))
        .unwrap_or(DEFAULT_TERMINAL_WIDTH);

    let mut draw_load_average = |cpus: &[cpu::CPU]| -> Result<(), ScannerError> {
        let load_average = load_average::get_load_average()?;

        let logical_cores_number: usize = cpus.iter().map(|cpu| cpu.siblings).sum();
        let logical_cores_number_f64 = logical_cores_number as f64;

        let one = format!("{:.2}", load_average.one);
        let five = format!("{:.2}", load_average.five);
        let fifteen = format!("{:.2}", load_average.fifteen);

        let load_average_len = one.len().max(five.len()).max(fifteen.len());

        let one_percentage =
            format!("{:.2}%", load_average.one * 100f64 / logical_cores_number_f64);
        let five_percentage =
            format!("{:.2}%", load_average.five * 100f64 / logical_cores_number_f64);
        let fifteen_percentage =
            format!("{:.2}%", load_average.fifteen * 100f64 / logical_cores_number_f64);

        let percentage_len =
            one_percentage.len().max(five_percentage.len()).max(fifteen_percentage.len());

        let progress_max = terminal_width - 11 - load_average_len - 2 - percentage_len - 1;

        // number of logical CPU cores

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        if logical_cores_number > 1 {
            write!(&mut stdout, "There are ")?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            write!(&mut stdout, "{}", logical_cores_number)?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, " logical CPU cores.")?;
        } else {
            write!(&mut stdout, "There is only one logical CPU core.")?;
        }
        writeln!(&mut stdout)?;

        // one

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "one    ")?; // 7

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.one * f).floor() as usize).min(progress_max);

        stdout.set_color(&*COLOR_USED)?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - one.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        stdout.write_all(one.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - one_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(one_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout)?;

        // five

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "five   ")?; // 7

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.five * f).floor() as usize).min(progress_max);

        stdout.set_color(&*COLOR_USED)?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - five.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        stdout.write_all(five.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - five_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(five_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout)?;

        // fifteen

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "fifteen")?; // 7

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.fifteen * f).floor() as usize).min(progress_max);

        stdout.set_color(&*COLOR_USED)?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - fifteen.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        stdout.write_all(fifteen.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - fifteen_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(fifteen_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout)?;
        writeln!(&mut stdout)?;

        Ok(())
    };

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            cpu::get_all_cpu_utilization_in_percentage(false, match monitor {
                Some(monitor) => monitor,
                None => Duration::from_millis(DEFAULT_INTERVAL),
            })?
        };

        let cpus = cpu::get_cpus()?;

        draw_load_average(&cpus)?;

        let mut i = 0;

        let cpus_len_dec = cpus.len() - 1;

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            writeln!(&mut stdout)?;

            let mut hz_string: Vec<String> = Vec::with_capacity(cpu.siblings);

            for cpu_mhz in cpu.cpus_mhz.iter().copied() {
                let cpu_hz =
                    Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                hz_string.push(format!(
                    "{:.2} {}Hz",
                    cpu_hz.get_value(),
                    &cpu_hz.get_unit().as_str()[..1]
                ));
            }

            let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap();

            // The max length of `CPU<number> `.
            let d = {
                let mut n = cpu.siblings;

                let mut d = 1;

                while n > 10 {
                    n /= 10;

                    d += 1;
                }

                d + 4
            };

            if only_information {
                for (i, hz_string) in hz_string.into_iter().enumerate() {
                    stdout.set_color(&*COLOR_LABEL)?;
                    write!(&mut stdout, "{1:<0$}", d, format!("CPU{}", i))?;

                    stdout.set_color(&*COLOR_BOLD_TEXT)?;
                    write!(&mut stdout, "{1:>0$}", hz_string_len, hz_string)?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout)?;
                }
            } else {
                let mut percentage_string: Vec<String> = Vec::with_capacity(cpu.siblings);

                for p in all_percentage[i..].iter().copied().take(cpu.siblings) {
                    percentage_string.push(format!("{:.2}%", p * 100f64));
                }

                let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap();

                let progress_max = terminal_width - d - 3 - percentage_len - 2 - hz_string_len - 1;

                let mut percentage_string_iter = percentage_string.into_iter();
                let mut hz_string_iter = hz_string.into_iter();

                for (i, p) in all_percentage[i..].iter().take(cpu.siblings).enumerate() {
                    let percentage_string = percentage_string_iter.next().unwrap();
                    let hz_string = hz_string_iter.next().unwrap();

                    stdout.set_color(&*COLOR_LABEL)?;
                    write!(&mut stdout, "{1:<0$}", d, format!("CPU{}", i))?;

                    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
                    write!(&mut stdout, "[")?; // 1

                    let f = progress_max as f64;

                    let progress_used = (p * f).floor() as usize;

                    stdout.set_color(&*COLOR_USED)?;
                    for _ in 0..progress_used {
                        write!(&mut stdout, "|")?; // 1
                    }

                    for _ in 0..(progress_max - progress_used) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
                    write!(&mut stdout, "] ")?; // 2

                    for _ in 0..(percentage_len - percentage_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(&*COLOR_BOLD_TEXT)?;
                    stdout.write_all(percentage_string.as_bytes())?;

                    write!(&mut stdout, " (")?; // 2

                    for _ in 0..(hz_string_len - hz_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.write_all(hz_string.as_bytes())?;

                    write!(&mut stdout, ")")?; // 1

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout)?;
                }

                i += cpu.siblings;
            }

            if cpu_index != cpus_len_dec {
                writeln!(&mut stdout)?;
            }
        }
    } else {
        let (average_percentage, average_percentage_string) = if only_information {
            (0f64, "".to_string())
        } else {
            let average_percentage =
                cpu::get_average_cpu_utilization_in_percentage(match monitor {
                    Some(monitor) => monitor,
                    None => Duration::from_millis(DEFAULT_INTERVAL),
                })?;

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        let cpus = cpu::get_cpus()?;

        draw_load_average(&cpus)?;

        for cpu in cpus {
            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            write!(&mut stdout, " ")?;

            let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

            let cpu_hz =
                Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

            write!(&mut stdout, "{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1])?;

            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;
        }

        if !only_information {
            let progress_max = terminal_width - 7 - average_percentage_string.len();

            stdout.set_color(&*COLOR_LABEL)?;
            write!(&mut stdout, "CPU")?; // 3

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64;

            let progress_used = (average_percentage * f).floor() as usize;

            stdout.set_color(&*COLOR_USED)?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            write!(&mut stdout, "] ")?; // 2

            stdout.set_color(&*COLOR_BOLD_TEXT)?;
            stdout.write_all(average_percentage_string.as_bytes())?;

            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout)?;
        }
    }

    output.print(&stdout)?;

    Ok(())
}

fn handle_time(monitor: bool) -> Result<(), Box<dyn Error>> {
    monitor_handler!(monitor, 1000, draw_time()?);
}

fn draw_time() -> Result<(), ScannerError> {
    let rtc_date_time = rtc_time::get_rtc_date_time()?;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "RTC Date")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    write!(&mut stdout, "{}", rtc_date_time.date())?;

    writeln!(&mut stdout)?;

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "RTC Time")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    write!(&mut stdout, "{}", rtc_date_time.time())?;

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout)?;

    output.print(&stdout)?;

    Ok(())
}

fn handle_uptime(monitor: bool, second: bool) -> Result<(), Box<dyn Error>> {
    monitor_handler!(monitor, 1000, draw_uptime(second)?);
}

fn draw_uptime(second: bool) -> Result<(), ScannerError> {
    let uptime = uptime::get_uptime()?.total_uptime;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, "This computer has been up for ")?;

    if second {
        let uptime_sec = uptime.as_secs();

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        write!(&mut stdout, "{} second", uptime_sec)?;

        if uptime_sec > 1 {
            write!(&mut stdout, "s")?;
        }
    } else {
        let s = format_duration(uptime);

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        stdout.write_all(s.as_bytes())?;
    }

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, ".")?;

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout)?;

    output.print(&stdout)?;

    Ok(())
}

#[inline]
fn handle_hostname() -> Result<(), Box<dyn Error>> {
    let hostname = hostname::get_hostname()?;

    println!("{}", hostname);

    Ok(())
}

#[inline]
fn handle_kernel() -> Result<(), Box<dyn Error>> {
    let kernel_version = kernel::get_kernel_version()?;

    println!("{}", kernel_version);

    Ok(())
}

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(APP_NAME)
        .set_term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("M Prober is a free and simple probe utility for Linux.\n\nEXAMPLES:\n", concat_line!(prefix "mprober ",
                "hostname                    # Show the hostname",
                "kernel                      # Show the kernel version",
                "uptime                      # Show the uptime",
                "uptime -m                   # Show the uptime and refresh every second",
                "uptime -p                   # Show the uptime without colors",
                "uptime -l                   # Show the uptime with darker colors (fitting in with light themes)",
                "uptime -s                   # Show the uptime in seconds",
                "time                        # Show the RTC (UTC) date and time",
                "time -m                     # Show the RTC (UTC) date and time and refresh every second",
                "time -p                     # Show the RTC (UTC) date and time without colors",
                "time -l                     # Show the RTC (UTC) date and time with darker colors (fitting in with light themes)",
                "cpu                         # Show load average and current CPU stats on average",
                "cpu -m 1000                 # Show load average and CPU stats on average and refresh every 1000 milliseconds",
                "cpu -p                      # Show load average and current CPU stats on average without colors",
                "cpu -l                      # Show load average and current CPU stats on average with darker colors (fitting in with light themes)",
                "cpu -s                      # Show load average and current stats of CPU cores separately",
                "cpu -i                      # Only show CPU information",
                "memory                      # Show current memory stats",
                "memory -m 1000              # Show memory stats and refresh every 1000 milliseconds",
                "memory -p                   # Show current memory stats without colors",
                "memory -l                   # Show current memory stats with darker colors (fitting in with light themes)",
                "memory -u kb                # Show current memory stats in KB",
                "network                     # Show current network stats",
                "network -m 1000             # Show network stats and refresh every 1000 milliseconds",
                "network -p                  # Show current network stats without colors",
                "network -l                  # Show current network stats with darker colors (fitting in with light themes)",
                "network -u kb               # Show current network stats in KB",
                "volume                      # Show current volume stats",
                "volume -m 1000              # Show current volume stats and refresh every 1000 milliseconds",
                "volume -p                   # Show current volume stats without colors",
                "volume -l                   # Show current volume stats without colors",
                "volume -u kb                # Show current volume stats in KB",
                "volume -i                   # Only show volume information without I/O rates",
                "volume --mounts             # Show current volume stats including mount points",
                "process                     # Show a snapshot of the current processes",
                "process -m 1000             # Show a snapshot of the current processes and refresh every 1000 milliseconds",
                "process -p                  # Show a snapshot of the current processes without colors",
                "process -l                  # Show a snapshot of the current processes with darker colors (fitting in with light themes)",
                "process -i                  # Show a snapshot of the current processes but not including CPU usage",
                "process -u kb               # Show a snapshot of the current processes. Information about memory size is in KB",
                "process --truncate 10       # Show a snapshot of the current processes with a specific truncation length to truncate user, group, program's names",
                "process --top 10            # Show a snapshot of current top-10 (ordered by CPU and memory usage) processes",
                "process -t                  # Show a snapshot of the current processes with the start time of each process",
                "process --pid-filter 3456   # Show a snapshot of the current processes which are related to a specific PID",
                "process --user-filter user1 # Show a snapshot of the current processes which are related to a specific user",
                "process --group-filter gp1  # Show a snapshot of the current processes which are related to a specific group",
                "process --tty-filter tty    # Show a snapshot of the current processes which are related to specific tty names matched by a regex",
                "process --program-filter ab # Show a snapshot of the current processes which are related to specific program names or commands matched by a regex",
                "web                         # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 3 seconds",
                "web -m 2                    # Start a HTTP service on port 8000 to monitor this computer. The time interval is set to 2 seconds",
                "web -p 7777                 # Start a HTTP service on port 7777 to monitor this computer",
                "web --addr 127.0.0.1        # Start a HTTP service on 127.0.0.1:8000 to monitor this computer",
                "web -a auth_key             # Start a HTTP service on port 8000 to monitor this computer. APIs need to be invoked with an auth key",
                "web --only-api              # Start a HTTP service on port 8000 to serve only HTTP APIs",
                "benchmark                   # Run benchmarks",
                "benchmark --disable-cpu     # Run benchmarks except for benchmarking CPU",
                "benchmark --enable-memory   # Benchmark the memory",
            )))
        .subcommand(
            SubCommand::with_name("hostname")
                .aliases(&["h", "host", "name", "servername"])
                .about("Shows the hostname")
                .display_order(0)
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("kernel")
                .aliases(&["k", "l", "linux"])
                .about("Shows the kernel version")
                .display_order(1)
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("uptime")
                .aliases(&["u", "up", "utime", "ut"])
                .about("Shows the uptime")
                .display_order(2)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows the uptime and refreshes every second"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("SECOND")
                        .long("second")
                        .short("s")
                        .help("Shows the uptime in seconds"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("time")
                .aliases(&[
                    "t", "systime", "stime", "st", "utc", "utctime", "rtc", "rtctime", "date",
                ])
                .about("Shows the RTC (UTC) date and time")
                .display_order(3)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows the RTC (UTC) date and time, and refreshes every second"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("cpu")
                .aliases(&["c", "cpus", "core", "cores", "load", "processor", "processors"])
                .about("Shows CPU stats")
                .display_order(4)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows CPU stats and refreshes every N milliseconds")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("SEPARATE")
                        .long("separate")
                        .short("s")
                        .help("Separates each CPU"),
                )
                .arg(
                    Arg::with_name("ONLY_INFORMATION")
                        .long("only-information")
                        .short("i")
                        .help("Shows only information about CPUs"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("memory")
                .aliases(&[
                    "m", "mem", "f", "free", "memories", "swap", "ram", "dram", "ddr", "cache",
                    "buffer", "buffers", "buf", "buff",
                ])
                .about("Shows memory stats")
                .display_order(5)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows memory stats and refreshes every N milliseconds")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("UNIT")
                        .long("unit")
                        .short("u")
                        .help("Forces to use a fixed unit")
                        .takes_value(true),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("network")
                .aliases(&["n", "net", "networks", "bandwidth", "traffic"])
                .about("Shows network stats")
                .display_order(6)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows network stats and refreshes every N milliseconds")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("UNIT")
                        .long("unit")
                        .short("u")
                        .help("Forces to use a fixed unit")
                        .takes_value(true),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("volume")
                .aliases(&[
                    "v", "storage", "volumes", "d", "disk", "disks", "blk", "block", "blocks",
                    "mount", "mounts", "ssd", "hdd",
                ])
                .about("Shows volume stats")
                .display_order(7)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows volume stats and refreshes every N milliseconds")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("UNIT")
                        .long("unit")
                        .short("u")
                        .help("Forces to use a fixed unit")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("ONLY_INFORMATION")
                        .long("only-information")
                        .short("i")
                        .help("Shows only information about volumes without I/O rates"),
                )
                .arg(
                    Arg::with_name("MOUNTS")
                        .long("mounts")
                        .aliases(&["mount", "point", "points"])
                        .help("Also shows mount points"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("process")
                .aliases(&["p", "ps"])
                .about("Shows process stats")
                .display_order(8)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Shows volume stats and refreshes every N milliseconds")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS"),
                )
                .arg(Arg::with_name("PLAIN").long("plain").short("p").help("No colors"))
                .arg(Arg::with_name("LIGHT").long("light").short("l").help("Darker colors"))
                .arg(
                    Arg::with_name("UNIT")
                        .long("unit")
                        .short("u")
                        .help("Forces to use a fixed unit")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("ONLY_INFORMATION")
                        .long("only-information")
                        .short("i")
                        .help("Shows only information about processes without CPU usage"),
                )
                .arg(
                    Arg::with_name("TOP")
                        .long("top")
                        .help("Sets the max number of processes shown on the screen")
                        .takes_value(true)
                        .value_name("MAX_NUMBER_OF_PROCESSES"),
                )
                .arg(
                    Arg::with_name("TRUNCATE")
                        .long("truncate")
                        .help("Truncates the user name, the group name and the program name of processes")
                        .takes_value(true)
                        .value_name("LENGTH")
                        .default_value("7"),
                )
                .arg(
                    Arg::with_name("START_TIME")
                        .long("start-time")
                        .short("t")
                        .alias("time")
                        .help("Shows when the progresses start")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("USER_FILTER")
                        .long("user-filter")
                        .alias("filter-user")
                        .help("Shows only processes which are related to a specific user")
                        .takes_value(true)
                        .value_name("USER_NAME"),
                )
                .arg(
                    Arg::with_name("GROUP_FILTER")
                        .long("group-filter")
                        .alias("filter-group")
                        .help("Shows only processes which are related to a specific group")
                        .takes_value(true)
                        .value_name("GROUP_NAME"),
                )
                .arg(
                    Arg::with_name("PROGRAM_FILTER")
                        .long("program-filter")
                        .alias("filter-program")
                        .help("Shows only processes which are related to specific programs or commands matched by a regex")
                        .takes_value(true)
                        .value_name("REGEX"),
                )
                .arg(
                    Arg::with_name("TTY_FILTER")
                        .long("tty-filter")
                        .alias("filter-tty")
                        .help("Shows only processes which are run on specific TTY/PTS matched by a regex")
                        .takes_value(true)
                        .value_name("REGEX"),
                )
                .arg(
                    Arg::with_name("PID_FILTER")
                        .long("pid-filter")
                        .alias("filter-pid")
                        .help("Shows only processes which are related to a specific PID")
                        .takes_value(true)
                        .value_name("PID"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("web")
                .aliases(&["w", "server", "http"])
                .about("Starts a HTTP service to monitor this computer")
                .display_order(9)
                .arg(
                    Arg::with_name("MONITOR")
                        .long("monitor")
                        .short("m")
                        .help("Automatically refreshes every N seconds")
                        .takes_value(true)
                        .value_name("SECONDS")
                        .default_value("3"),
                )
                .arg(
                    Arg::with_name("ADDRESS")
                        .long("address")
                        .visible_alias("addr")
                        .help("Assigns the address that M Prober binds.")
                        .takes_value(true)
                        .default_value(if cfg!(debug_assertions) {
                            "127.0.0.1"
                        } else {
                            "0.0.0.0"
                        }),
                )
                .arg(
                    Arg::with_name("LISTEN_PORT")
                        .long("listen-port")
                        .visible_alias("port")
                        .short("p")
                        .help("Assigns a TCP port for the HTTP service")
                        .takes_value(true)
                        .default_value("8000"),
                )
                .arg(
                    Arg::with_name("AUTH_KEY")
                        .long("auth-key")
                        .short("a")
                        .help("Assigns an auth key")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("ONLY_API")
                        .long("only-api")
                        .aliases(&["only-apis"])
                        .help("Disables the web page"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .subcommand(
            SubCommand::with_name("benchmark")
                .aliases(&["b", "bench", "performance"])
                .about("Runs benchmarks to measure the performance of this environment")
                .display_order(10)
                .arg(
                    Arg::with_name("WARMING_UP_DURATION")
                        .display_order(0)
                        .long("warming-up-duration")
                        .help("Assigns a duration for warming up")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS")
                        .default_value("3000"),
                )
                .arg(
                    Arg::with_name("BENCHMARK_DURATION")
                        .display_order(1)
                        .long("benchmark-duration")
                        .help("Assigns a duration for each benchmarking")
                        .takes_value(true)
                        .value_name("MILLI_SECONDS")
                        .default_value("5000"),
                )
                .arg(
                    Arg::with_name("VERBOSE")
                        .display_order(2)
                        .long("verbose")
                        .short("v")
                        .help("Shows more information in stderr"),
                )
                .arg(
                    Arg::with_name("DISABLE_CPU")
                        .display_order(10)
                        .long("disable-cpu")
                        .aliases(&["disabled-cpu", "disable-cpus", "disabled-cpus"])
                        .help("Not to benchmark CPUs"),
                )
                .arg(
                    Arg::with_name("ENABLE_CPU")
                        .display_order(100)
                        .long("enable-cpu")
                        .aliases(&["enabled-cpu", "enable-cpus", "enabled-cpus"])
                        .help("Allows to benchmark CPUs (disables others by default)"),
                )
                .arg(
                    Arg::with_name("DISABLE_MEMORY")
                        .display_order(11)
                        .long("disable-memory")
                        .aliases(&["disabled-memory"])
                        .help("Not to benchmark memory"),
                )
                .arg(
                    Arg::with_name("ENABLE_MEMORY")
                        .display_order(101)
                        .long("enable-memory")
                        .aliases(&["enabled-memory"])
                        .help("Allows to benchmark memory (disables others by default)"),
                )
                .arg(
                    Arg::with_name("DISABLE_VOLUME")
                        .display_order(13)
                        .long("disable-volume")
                        .aliases(&["disabled-volume", "disable-volumes", "disabled-volumes"])
                        .help("Not to benchmark volumes"),
                )
                .arg(
                    Arg::with_name("ENABLE_VOLUME")
                        .display_order(103)
                        .long("enable-volume")
                        .aliases(&["enabled-volume", "enable-volumes", "enabled-volumes"])
                        .help("Allows to benchmark volumes (disables others by default)"),
                )
                .after_help("Enjoy it! https://magiclen.org"),
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches()
}

#[inline]
fn get_monitor_duration(matches: &ArgMatches) -> Result<Option<Duration>, Box<dyn Error>> {
    match matches.value_of("MONITOR") {
        Some(monitor) => {
            let monitor = MonitorInterval::parse_str(monitor)
                .map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?
                .get_number();

            Ok(Some(Duration::from_secs_f64(monitor / 1000f64)))
        }
        None => Ok(None),
    }
}

#[inline]
fn get_byte_unit(matches: &ArgMatches) -> Result<Option<ByteUnit>, Box<dyn Error>> {
    match matches.value_of("UNIT") {
        Some(unit) => {
            let unit = ByteUnit::from_str(unit)
                .map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

            Ok(Some(unit))
        }
        None => Ok(None),
    }
}
