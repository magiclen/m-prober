#![feature(duration_float)]
#![feature(proc_macro_hygiene, decl_macro)]

extern crate clap;
extern crate byte_unit;
#[macro_use]
extern crate validators;
extern crate termcolor;
extern crate terminal_size;
extern crate getch;
extern crate scanner_rust;
extern crate libc;

extern crate rand;
extern crate base64;
#[macro_use]
extern crate enum_ordinalize;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_simple_authorization;
extern crate rocket_cache_response;
extern crate rocket_json_response;

mod free;
mod cpu_info;
mod load_average;
mod hostname;
mod time;
mod kernel;
mod network;
mod disk;
mod rocket_mounts;

use std::time::Duration;
use std::env;
use std::path::Path;
use std::io::Write;
use std::thread;
use std::fmt::Write as WriteFmt;
use std::process;

use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use terminal_size::{Width, terminal_size};
use clap::{App, Arg, SubCommand};
use validators::number::NumberGtZero;
use byte_unit::{Byte, ByteUnit};
use getch::Getch;
use scanner_rust::ScannerError;

use free::Free;
use cpu_info::{CPU, CPUStat};
use load_average::LoadAverage;
use time::RTCDateTime;
use network::NetworkWithSpeed;
use disk::{Disk, DiskWithSpeed};

const DEFAULT_TERMINAL_WIDTH: usize = 64;
const MIN_TERMINAL_WIDTH: usize = 60;
const DEFAULT_INTERVAL: u64 = 333; // should be smaller than 1000 milliseconds

const LABEL_COLOR: Color = Color::Rgb(0, 177, 177);
const WHITE_COLOR: Color = Color::Rgb(219, 219, 219);
const RED_COLOR: Color = Color::Rgb(255, 95, 0);
const YELLOW_COLOR: Color = Color::Rgb(216, 177, 0);
const SKY_BLUE_COLOR: Color = Color::Rgb(107, 200, 200);

const CLEAR_SCREEN_DATA: [u8; 11] = [0x1b, 0x5b, 0x33, 0x4a, 0x1b, 0x5b, 0x48, 0x1b, 0x5b, 0x32, 0x4a];

// TODO -----Config START-----

#[derive(Debug)]
pub enum Mode {
    HostName,
    Kernel,
    Uptime {
        monitor: bool,
        plain: bool,
        second: bool,
    },
    Time {
        monitor: bool,
        plain: bool,
    },
    CPU {
        monitor: Option<Duration>,
        plain: bool,
        separate: bool,
        information: bool,
    },
    Memory {
        monitor: Option<Duration>,
        plain: bool,
        unit: Option<ByteUnit>,
    },
    Network {
        monitor: Option<Duration>,
        plain: bool,
        unit: Option<ByteUnit>,
    },
    Disk {
        monitor: Option<Duration>,
        plain: bool,
        unit: Option<ByteUnit>,
        information: bool,
        mounts: bool,
    },
    Web {
        monitor: Duration,
        port: u16,
        auth_key: Option<String>,
    },
}

#[derive(Debug)]
pub struct Config {
    pub mode: Mode
}

const APP_NAME: &str = "M Prober (MagicLen Prober)";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();
        let arg0 = Path::new(&arg0).file_name().unwrap().to_str().ok_or("The file name of this program is not supported by UTF-8.".to_string())?;

        let examples = vec![
            "hostname                    # Show the hostname",
            "kernel                      # Show the kernel version",
            "uptime                      # Show the uptime",
            "uptime -m                   # Show the uptime and refresh every second",
            "uptime -p                   # Show the uptime without colors",
            "uptime -s                   # Show the uptime in seconds",
            "time                        # Show the RTC (UTC) date and time",
            "time -m                     # Show the RTC (UTC) date and time and refresh every second",
            "time -p                     # Show the RTC (UTC) date and time without colors",
            "cpu                         # Show load average and current CPU stats on average",
            "cpu -m 1000                 # Show load average and CPU stats on average and refresh every 1000 milliseconds",
            "cpu -p                      # Show load average and current CPU stats on average without colors",
            "cpu -s                      # Show load average and current stats of CPU cores separately",
            "cpu -i                      # Only show CPU information",
            "memory                      # Show current memory stats",
            "memory -m 1000              # Show memory stats and refresh every 1000 milliseconds",
            "memory -p                   # Show current memory stats without colors",
            "memory -u kb                # Show current memory stats in KB",
            "network                     # Show current network stats",
            "network -m 1000             # Show network stats and refresh every 1000 milliseconds",
            "network -p                  # Show current network stats without colors",
            "network -u kb               # Show current network stats in KB",
            "disk                        # Show current disk stats",
            "disk -m 1000                # Show current disk stats and refresh every 1000 milliseconds",
            "disk -p                     # Show current disk stats without colors",
            "disk -u kb                  # Show current disk stats in KB",
            "disk -i                     # Only show disk information without I/O rates",
            "disk --mounts               # Show current disk stats including mount points",
            "web                         # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 1 second.",
            "web -m 2                    # Start a HTTP service on port 8000 to monitor this computer. The time interval is set to 2 seconds.",
            "web -p 7777                 # Start a HTTP service on port 7777 to monitor this computer.",
            "web -a auth_key             # Start a HTTP service on port 8000 to monitor this computer. APIs need to be invoked with an auth key.",
        ];

        let terminal_width = if let Some((Width(width), _)) = terminal_size() {
            width as usize
        } else {
            0
        };

        let matches = App::new(APP_NAME)
            .set_term_width(terminal_width)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about(format!("MagicLen Prober is a free and simple probe utility for Linux.\n\nEXAMPLES:\n{}", examples.iter()
                .map(|e| format!("  {} {}\n", arg0, e))
                .collect::<Vec<String>>()
                .concat()
            ).as_str()
            )
            .subcommand(SubCommand::with_name("hostname").aliases(&["h", "host", "name", "servername", "server"])
                .about("Shows the hostname")
                .display_order(0)
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("kernel").aliases(&["k", "l", "linux"])
                .about("Shows the kernel version")
                .display_order(1)
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("uptime").aliases(&["u", "up", "utime", "ut"])
                .about("Shows the uptime")
                .display_order(2)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows the uptime and refreshes every second")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("SECOND")
                    .long("second")
                    .short("s")
                    .help("Shows the uptime in seconds")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("time").aliases(&["t", "systime", "stime", "st", "utc", "utctime", "rtc", "rtctime", "date"])
                .about("Shows the RTC (UTC) date and time")
                .display_order(3)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows the RTC (UTC) date and time, and refreshes every second")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("cpu").aliases(&["c", "cpus", "core", "cores", "load", "processor", "processors"])
                .about("Shows CPU stats")
                .display_order(4)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows CPU stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("SEPARATE")
                    .long("separate")
                    .short("s")
                    .help("Separates each CPU")
                )
                .arg(Arg::with_name("INFORMATION")
                    .long("information")
                    .short("i")
                    .help("Shows only information about CPUs")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("memory").aliases(&["m", "mem", "free", "memories", "swap", "ram", "dram", "ddr", "cache", "buffer", "buf"])
                .about("Shows memory stats")
                .display_order(5)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows memory stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("UNIT")
                    .long("unit")
                    .short("u")
                    .help("Forces to use a fixed unit")
                    .takes_value(true)
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("network").aliases(&["n", "net", "networks", "bandwidth", "traffic"])
                .about("Shows network stats")
                .display_order(6)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows network stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("UNIT")
                    .long("unit")
                    .short("u")
                    .help("Forces to use a fixed unit")
                    .takes_value(true)
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("disk").aliases(&["d", "storage", "disks", "blk", "block", "blocks", "mount", "mounts", "ssd", "hdd"])
                .about("Shows disk stats")
                .display_order(7)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows disk stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("UNIT")
                    .long("unit")
                    .short("u")
                    .help("Forces to use a fixed unit")
                    .takes_value(true)
                )
                .arg(Arg::with_name("INFORMATION")
                    .long("information")
                    .short("i")
                    .help("Shows only information about disks without I/O rates")
                )
                .arg(Arg::with_name("MOUNTS")
                    .long("mounts")
                    .aliases(&["mount", "point", "points"])
                    .help("Also shows mount points")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("web").aliases(&["server", "http"])
                .about("Starts a HTTP service to monitor this computer")
                .display_order(8)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Automatically refreshes every N seconds")
                    .takes_value(true)
                    .value_name("SECONDS")
                    .default_value("1")
                )
                .arg(Arg::with_name("PORT")
                    .long("port")
                    .short("p")
                    .help("Assigns a TCP port for the HTTP service")
                    .takes_value(true)
                    .default_value("8000")
                )
                .arg(Arg::with_name("AUTH_KEY")
                    .long("auth-key")
                    .short("a")
                    .help("Assigns an auth key")
                    .takes_value(true)
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let mode = if let Some(_) = matches.subcommand_matches("hostname") {
            Mode::HostName
        } else if let Some(_) = matches.subcommand_matches("kernel") {
            Mode::Kernel
        } else if let Some(sub_matches) = matches.subcommand_matches("uptime") {
            let monitor = sub_matches.is_present("MONITOR");

            let plain = sub_matches.is_present("PLAIN");

            let second = sub_matches.is_present("SECOND");

            Mode::Uptime {
                monitor,
                plain,
                second,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("time") {
            let monitor = sub_matches.is_present("MONITOR");

            let plain = sub_matches.is_present("PLAIN");

            Mode::Time {
                monitor,
                plain,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("cpu") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let separate = sub_matches.is_present("SEPARATE");

            let information = sub_matches.is_present("INFORMATION");

            Mode::CPU {
                monitor,
                plain,
                separate,
                information,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("memory") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            Mode::Memory {
                monitor,
                plain,
                unit,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("network") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            Mode::Network {
                monitor,
                plain,
                unit,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("disk") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            let information = sub_matches.is_present("INFORMATION");

            let mounts = sub_matches.is_present("MOUNTS");

            Mode::Disk {
                monitor,
                plain,
                unit,
                information,
                mounts,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("web") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor: u64 = monitor.parse().map_err(|_| format!("`{}` is not a correct value for SECONDS", monitor))?;

                    if monitor == 0 {
                        return Err(format!("`{}` is not a correct value for SECONDS", monitor));
                    }

                    Duration::from_secs(monitor)
                }
                None => unreachable!()
            };

            let port = match sub_matches.value_of("PORT") {
                Some(port) => {
                    let port: u16 = port.parse().map_err(|_| format!("`{}` is not a correct value for PORT", port))?;

                    port
                }
                None => unreachable!()
            };

            let auth_key = sub_matches.value_of("AUTH_KEY").map(|s| s.to_string());

            Mode::Web {
                monitor,
                port,
                auth_key,
            }
        } else {
            return Err(String::from("Please input a subcommand. Use `help` to see how to use this program."));
        };

        let config = Config {
            mode
        };

        Ok(config)
    }
}

// TODO -----Config END----

pub fn run(config: Config) -> Result<i32, String> {
    let mode = config.mode;

    match mode {
        Mode::HostName => {
            let hostname = hostname::get_hostname().map_err(|err| err.to_string())?;

            println!("{}", hostname);
        }
        Mode::Kernel => {
            let hostname = kernel::get_kernel_version().map_err(|err| err.to_string())?;

            println!("{}", hostname);
        }
        Mode::Uptime { monitor, plain, second } => {
            if monitor {
                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        match key {
                            b'q' => {
                                break;
                            }
                            _ => ()
                        }
                    }

                    process::exit(0);
                });

                let sleep_interval = Duration::from_secs(1);

                loop {
                    draw_uptime(!plain, second, true).map_err(|err| err.to_string())?;

                    thread::sleep(sleep_interval);
                }
            } else {
                draw_uptime(!plain, second, false).map_err(|err| err.to_string())?;
            }
        }
        Mode::Time { monitor, plain } => {
            if monitor {
                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        match key {
                            b'q' => {
                                break;
                            }
                            _ => ()
                        }
                    }

                    process::exit(0);
                });

                let sleep_interval = Duration::from_secs(1);

                loop {
                    draw_time(!plain, true).map_err(|err| err.to_string())?;

                    thread::sleep(sleep_interval);
                }
            } else {
                draw_time(!plain, false).map_err(|err| err.to_string())?;
            }
        }
        Mode::CPU { monitor, plain, separate, information } => {
            match monitor {
                Some(monitor) => {
                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        process::exit(0);
                    });

                    draw_cpu_info(!plain, separate, information, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        if information {
                            thread::sleep(sleep_interval);
                        }

                        draw_cpu_info(!plain, separate, information, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_cpu_info(!plain, separate, information, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Memory { monitor, plain, unit } => {
            match monitor {
                Some(monitor) => {
                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        process::exit(0);
                    });

                    let sleep_interval = monitor;

                    loop {
                        draw_memory(!plain, unit, true).map_err(|err| err.to_string())?;

                        thread::sleep(sleep_interval);
                    }
                }
                None => {
                    draw_memory(!plain, unit, false).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Network { monitor, plain, unit } => {
            match monitor {
                Some(monitor) => {
                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        process::exit(0);
                    });

                    draw_network(!plain, unit, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        draw_network(!plain, unit, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_network(!plain, unit, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Disk { monitor, plain, unit, information, mounts } => {
            match monitor {
                Some(monitor) => {
                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        process::exit(0);
                    });

                    draw_disk(!plain, unit, information, mounts, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        if information {
                            thread::sleep(sleep_interval);
                        }

                        draw_disk(!plain, unit, information, mounts, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_disk(!plain, unit, information, mounts, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Web { monitor, port, auth_key } => {
            rocket_mounts::launch(monitor, port, auth_key);
        }
    }

    Ok(0)
}

fn draw_uptime(colorful: bool, second: bool, monitor: bool) -> Result<(), ScannerError> {
    let uptime = time::get_uptime()?;

    let uptime_sec = uptime.as_secs();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "This computer has been up for ")?;

    if second {
        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        write!(&mut stdout, "{} second", uptime_sec)?;

        if uptime_sec > 1 {
            write!(&mut stdout, "s")?;
        }
    } else {
        let days = uptime_sec / 86400;

        let uptime_sec = uptime_sec % 86400;

        let hours = uptime_sec / 3600;

        let uptime_sec = uptime_sec % 3600;

        let minutes = uptime_sec / 60;

        let seconds = uptime_sec % 60;

        let mut s = String::new();

        if days > 0 {
            s.write_fmt(format_args!("{} day", days)).unwrap();

            if days > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if hours > 0 || (days > 0) && (minutes > 0 || seconds > 0) {
            s.write_fmt(format_args!("{} hour", hours)).unwrap();

            if hours > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if minutes > 0 || (hours > 0 && seconds > 0) {
            s.write_fmt(format_args!("{} minute", minutes)).unwrap();

            if minutes > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if seconds > 0 {
            s.write_fmt(format_args!("{} second", seconds)).unwrap();

            if seconds > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        debug_assert!(s.len() >= 2);

        if let Some(index) = s.as_str()[..(s.len() - 2)].rfind(", ") {
            s.insert_str(index + 2, "and ");
        }

        let b = s.as_bytes();

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(&b[..(b.len() - 2)])?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, ".")?;

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_time(colorful: bool, monitor: bool) -> Result<(), ScannerError> {
    let rtc_date_time: RTCDateTime = RTCDateTime::get_rtc_date_time()?;

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "RTC Date")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(rtc_date_time.rtc_date.as_bytes())?;

    writeln!(&mut stdout, "")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "RTC Time")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(rtc_date_time.rtc_time.as_bytes())?;

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

// TODO

fn draw_cpu_info(colorful: bool, separate: bool, only_information: bool, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let cpus: Vec<CPU> = CPU::get_cpus()?;

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor.is_some() {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    let terminal_width = if let Some((Width(width), _)) = terminal_size() {
        (width as usize).max(MIN_TERMINAL_WIDTH)
    } else {
        DEFAULT_TERMINAL_WIDTH
    };

    // load average
    if !only_information {
        let load_average: LoadAverage = LoadAverage::get_load_average()?;

        let logical_cores_number: usize = cpus.iter().map(|cpu| cpu.siblings).sum();
        let logical_cores_number_f64 = logical_cores_number as f64;

        let one = format!("{:.2}", load_average.one);
        let five = format!("{:.2}", load_average.five);
        let fifteen = format!("{:.2}", load_average.fifteen);

        let load_average_len = one.len().max(five.len()).max(fifteen.len());

        let one_percentage = format!("{:.2}%", load_average.one * 100f64 / logical_cores_number_f64);
        let five_percentage = format!("{:.2}%", load_average.five * 100f64 / logical_cores_number_f64);
        let fifteen_percentage = format!("{:.2}%", load_average.fifteen * 100f64 / logical_cores_number_f64);

        let percentage_len = one_percentage.len().max(five_percentage.len()).max(fifteen_percentage.len());

        let progress_max = terminal_width - 11 - load_average_len - 2 - percentage_len - 1;

        // number of logical CPU cores

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        if logical_cores_number > 1 {
            write!(&mut stdout, "There are ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            write!(&mut stdout, "{}", logical_cores_number)?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " logical CPU cores.")?;
        } else {
            write!(&mut stdout, "There is only one logical CPU core.")?;
        }
        writeln!(&mut stdout, "")?;

        // one

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "one    ")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.one * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - one.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(one.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - one_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(one_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;

        // five

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "five   ")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.five * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - five.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(five.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - five_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(five_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;

        // fifteen

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "fifteen")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.fifteen * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - fifteen.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(fifteen.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - fifteen_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(fifteen_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;
        writeln!(&mut stdout, "")?;
    }

    // information

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            CPUStat::get_all_percentage(match monitor {
                Some(monitor) => monitor,
                None => Duration::from_millis(DEFAULT_INTERVAL)
            })?
        };

        let mut i = 0;

        let cpus_len_dec = cpus.len() - 1;

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            writeln!(&mut stdout, "")?;

            let mut hz_string: Vec<String> = Vec::with_capacity(cpu.siblings);

            for &cpu_mhz in cpu.cpus_mhz.iter() {
                let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                hz_string.push(format!("{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1]));
            }

            let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap();

            let d = {
                let mut n = cpu.siblings;

                let mut d = 1;

                while n > 10 {
                    n /= 10;

                    d += 1;
                }

                d
            };

            if only_information {
                for (i, hz_string) in hz_string.into_iter().enumerate() {
                    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
                    write!(&mut stdout, "{1:<0$}", d + 4, format!("CPU{}", i))?;

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
                    write!(&mut stdout, "{1:>0$}", hz_string_len, hz_string)?;

                    writeln!(&mut stdout, "")?;
                }
            } else {
                let mut percentage_string: Vec<String> = Vec::with_capacity(cpu.siblings);

                for &p in all_percentage[i..].iter().take(cpu.siblings) {
                    percentage_string.push(format!("{:.2}%", p * 100f64));
                }

                let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap();

                let progress_max = terminal_width - d - 7 - percentage_len - 2 - hz_string_len - 1;

                let mut percentage_string_iter = percentage_string.into_iter();
                let mut hz_string_iter = hz_string.into_iter();

                for p in all_percentage[i..].into_iter().take(cpu.siblings) {
                    let percentage_string = percentage_string_iter.next().unwrap();
                    let hz_string = hz_string_iter.next().unwrap();

                    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
                    write!(&mut stdout, "CPU")?; // 3

                    write!(&mut stdout, "{}", i)?;

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
                    write!(&mut stdout, " [")?; // 2

                    let f = progress_max as f64;

                    let progress_used = (p * f).floor() as usize;

                    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
                    for _ in 0..progress_used {
                        write!(&mut stdout, "|")?; // 1
                    }

                    for _ in 0..(progress_max - progress_used) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
                    write!(&mut stdout, "] ")?; // 2

                    for _ in 0..(percentage_len - percentage_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
                    stdout.write_all(percentage_string.as_bytes())?;

                    write!(&mut stdout, " (")?; // 2

                    for _ in 0..(hz_string_len - hz_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.write_all(hz_string.as_bytes())?;

                    write!(&mut stdout, ")")?; // 1

                    writeln!(&mut stdout, "")?;
                }

                i += cpu.siblings;
            }

            if cpu_index != cpus_len_dec {
                writeln!(&mut stdout, "")?;
            }
        }
    } else {
        let (average_percentage, average_percentage_string) = if only_information {
            (0f64, "".to_string())
        } else {
            let average_percentage = CPUStat::get_average_percentage(match monitor {
                Some(monitor) => monitor,
                None => Duration::from_millis(DEFAULT_INTERVAL)
            })?;

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        for cpu in cpus {
            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            write!(&mut stdout, " ")?;

            let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

            let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

            write!(&mut stdout, "{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1])?;

            writeln!(&mut stdout, "")?;
        }

        if !only_information {
            let progress_max = terminal_width - 7 - average_percentage_string.len();

            stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
            write!(&mut stdout, "CPU")?; // 3

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64;

            let progress_used = (average_percentage * f).floor() as usize;

            stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, "] ")?; // 2

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(average_percentage_string.as_bytes())?;

            writeln!(&mut stdout, "")?;
        }
    }

    output.print(&stdout)?;

    Ok(())
}

fn draw_memory(colorful: bool, unit: Option<ByteUnit>, monitor: bool) -> Result<(), ScannerError> {
    let free = Free::get_free()?;

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

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
    let swap_percentage = format!("{:.2}%", free.swap.used as f64 * 100f64 / free.swap.total as f64);

    let percentage_len = mem_percentage.len().max(swap_percentage.len());

    let terminal_width = if let Some((Width(width), _)) = terminal_size() {
        (width as usize).max(MIN_TERMINAL_WIDTH)
    } else {
        DEFAULT_TERMINAL_WIDTH
    };

    // Memory

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "Memory")?; // 6

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " [")?; // 2

    let progress_max = terminal_width - 10 - used_len - 3 - total_len - 2 - percentage_len - 1;

    let f = progress_max as f64 / free.mem.total as f64;

    let progress_used = (free.mem.used as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.mem.cache as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(YELLOW_COLOR)))?;
    for _ in 0..progress_cache {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "$")?; // 1
        }
    }

    let progress_buffers = (free.mem.buffers as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(SKY_BLUE_COLOR)))?;
    for _ in 0..progress_buffers {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "#")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache - progress_buffers) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - mem_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(mem_used.as_bytes())?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - mem_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(mem_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - mem_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(mem_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    writeln!(&mut stdout, "")?;

    // Swap

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "Swap  ")?; // 6

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " [")?; // 2

    let f = progress_max as f64 / free.swap.total as f64;

    let progress_used = (free.swap.used as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.swap.cache as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(YELLOW_COLOR)))?;
    for _ in 0..progress_cache {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "$")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - swap_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(swap_used.as_bytes())?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - swap_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(swap_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - swap_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(swap_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_network(colorful: bool, unit: Option<ByteUnit>, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let networks_with_speed = NetworkWithSpeed::get_networks_with_speed(match monitor {
        Some(monitor) => monitor,
        None => Duration::from_millis(DEFAULT_INTERVAL)
    })?;

    let networks_with_speed_len = networks_with_speed.len();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor.is_some() {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    debug_assert!(networks_with_speed_len > 0);

    let mut uploads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut uploads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    let mut downloads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut downloads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    for network_with_speed in networks_with_speed.iter() {
        let upload = Byte::from_unit(network_with_speed.speed.transmit, ByteUnit::B).unwrap();
        let upload_total = Byte::from_bytes(network_with_speed.network.transmit_bytes as u128);

        let download = Byte::from_unit(network_with_speed.speed.receive, ByteUnit::B).unwrap();
        let download_total = Byte::from_bytes(network_with_speed.network.receive_bytes as u128);

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

    let interface_len = networks_with_speed.iter().map(|network_with_sppeed| network_with_sppeed.network.interface.len()).max().unwrap();
    let interface_len_inc = interface_len + 1;

    let upload_len = uploads.iter().map(|upload| upload.len()).max().unwrap().max(11);
    let upload_total_len = uploads_total.iter().map(|upload_total| upload_total.len()).max().unwrap().max(13);
    let download_len = downloads.iter().map(|download| download.len()).max().unwrap().max(13);
    let download_total_len = downloads_total.iter().map(|download_total| download_total.len()).max().unwrap().max(15);

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "{1:>0$}", interface_len_inc + upload_len, "Upload Rate")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "{1:>0$}", upload_total_len, "Uploaded Data")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "{1:>0$}", download_len, "Download Rate")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " | ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "{1:>0$}", download_total_len, "Downloaded Data")?;

    writeln!(&mut stdout, "")?;

    let mut uploads_iter = uploads.into_iter();
    let mut uploads_total_iter = uploads_total.into_iter();
    let mut downloads_iter = downloads.into_iter();
    let mut downloads_total_iter = downloads_total.into_iter();

    for network_with_speed in networks_with_speed.into_iter() {
        let upload = uploads_iter.next().unwrap();
        let upload_total = uploads_total_iter.next().unwrap();

        let download = downloads_iter.next().unwrap();
        let download_total = downloads_total_iter.next().unwrap();

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:<0$}", interface_len_inc, network_with_speed.network.interface)?;

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

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

        writeln!(&mut stdout, "")?;
    }

    output.print(&stdout)?;

    Ok(())
}

fn draw_disk(colorful: bool, unit: Option<ByteUnit>, only_information: bool, mounts: bool, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor.is_some() {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    let terminal_width = if let Some((Width(width), _)) = terminal_size() {
        (width as usize).max(MIN_TERMINAL_WIDTH)
    } else {
        DEFAULT_TERMINAL_WIDTH
    };

    if only_information {
        let disks = Disk::get_disks()?;

        let disks_len = disks.len();

        debug_assert!(disks_len > 0);

        let mut disks_size: Vec<String> = Vec::with_capacity(disks_len);

        let mut disks_used: Vec<String> = Vec::with_capacity(disks_len);

        let mut disks_used_percentage: Vec<String> = Vec::with_capacity(disks_len);

        let mut disks_read_total: Vec<String> = Vec::with_capacity(disks_len);

        let mut disks_write_total: Vec<String> = Vec::with_capacity(disks_len);

        for disk in disks.iter() {
            let size = Byte::from_bytes(disk.size as u128);

            let used = Byte::from_bytes(disk.used as u128);

            let used_percentage = format!("{:.2}%", (disk.used * 100) as f64 / disk.size as f64);

            let read_total = Byte::from_bytes(disk.read_bytes as u128);

            let write_total = Byte::from_bytes(disk.write_bytes as u128);

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

            disks_size.push(size);
            disks_used.push(used);
            disks_used_percentage.push(used_percentage);
            disks_read_total.push(read_total);
            disks_write_total.push(write_total);
        }

        let devices_len = disks.iter().map(|disk| disk.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let disks_size_len = disks_size.iter().map(|size| size.len()).max().unwrap();
        let disks_used_len = disks_used.iter().map(|used| used.len()).max().unwrap();
        let disks_used_percentage_len = disks_used_percentage.iter().map(|used_percentage| used_percentage.len()).max().unwrap();
        let disks_read_total_len = disks_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let disks_write_total_len = disks_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width - devices_len - 4 - disks_used_len - 3 - disks_size_len - 2 - disks_used_percentage_len - 1;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", devices_len_inc + disks_read_total_len, "Read Data")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", disks_write_total_len, "Written Data")?;

        writeln!(&mut stdout, "")?;

        let mut disks_size_iter = disks_size.into_iter();
        let mut disks_used_iter = disks_used.into_iter();
        let mut disks_used_percentage_iter = disks_used_percentage.into_iter();
        let mut disks_read_total_iter = disks_read_total.into_iter();
        let mut disks_write_total_iter = disks_write_total.into_iter();

        for disk in disks.into_iter() {
            let size = disks_size_iter.next().unwrap();

            let used = disks_used_iter.next().unwrap();

            let used_percentage = disks_used_percentage_iter.next().unwrap();

            let read_total = disks_read_total_iter.next().unwrap();

            let write_total = disks_write_total_iter.next().unwrap();

            stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
            write!(&mut stdout, "{1:<0$}", devices_len_inc, disk.device)?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            for _ in 0..(disks_read_total_len - read_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read_total.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(disks_write_total_len - write_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write_total.as_bytes())?;

            writeln!(&mut stdout, "")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;

            for _ in 0..devices_len {
                write!(&mut stdout, " ")?;
            }

            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64 / disk.size as f64;

            let progress_used = (disk.used as f64 * f).floor() as usize;

            stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, "] ")?; // 2

            for _ in 0..(disks_used_len - used.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(used.as_bytes())?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " / ")?; // 3

            for _ in 0..(disks_size_len - size.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(size.as_bytes())?;

            write!(&mut stdout, " (")?; // 2

            for _ in 0..(disks_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.write_all(used_percentage.as_bytes())?;

            write!(&mut stdout, ")")?; // 1

            writeln!(&mut stdout, "")?;

            if mounts {
                stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;

                for point in disk.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    writeln!(&mut stdout, "")?;
                }
            }
        }
    } else {
        let disks_with_speed = DiskWithSpeed::get_disks_with_speed(match monitor {
            Some(monitor) => monitor,
            None => Duration::from_millis(DEFAULT_INTERVAL)
        })?;

        let disks_with_speed_len = disks_with_speed.len();

        debug_assert!(disks_with_speed_len > 0);

        let mut disks_size: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_used: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_used_percentage: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_read: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_read_total: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_write: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        let mut disks_write_total: Vec<String> = Vec::with_capacity(disks_with_speed_len);

        for disk_with_speed in disks_with_speed.iter() {
            let size = Byte::from_bytes(disk_with_speed.disk.size as u128);

            let used = Byte::from_bytes(disk_with_speed.disk.used as u128);

            let used_percentage = format!("{:.2}%", (disk_with_speed.disk.used * 100) as f64 / disk_with_speed.disk.size as f64);

            let read = Byte::from_unit(disk_with_speed.speed.read, ByteUnit::B).unwrap();
            let read_total = Byte::from_bytes(disk_with_speed.disk.read_bytes as u128);

            let write = Byte::from_unit(disk_with_speed.speed.write, ByteUnit::B).unwrap();
            let write_total = Byte::from_bytes(disk_with_speed.disk.write_bytes as u128);

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

            disks_size.push(size);
            disks_used.push(used);
            disks_used_percentage.push(used_percentage);
            disks_read.push(read);
            disks_read_total.push(read_total);
            disks_write.push(write);
            disks_write_total.push(write_total);
        }

        let devices_len = disks_with_speed.iter().map(|disk_with_speed| disk_with_speed.disk.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let disks_size_len = disks_size.iter().map(|size| size.len()).max().unwrap();
        let disks_used_len = disks_used.iter().map(|used| used.len()).max().unwrap();
        let disks_used_percentage_len = disks_used_percentage.iter().map(|used_percentage| used_percentage.len()).max().unwrap();
        let disks_read_len = disks_read.iter().map(|read| read.len()).max().unwrap().max(12);
        let disks_read_total_len = disks_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let disks_write_len = disks_write.iter().map(|write| write.len()).max().unwrap().max(12);
        let disks_write_total_len = disks_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width - devices_len - 4 - disks_used_len - 3 - disks_size_len - 2 - disks_used_percentage_len - 1;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", devices_len_inc + disks_read_len, "Reading Rate")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", disks_read_total_len, "Read Data")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", disks_write_len, "Writing Rate")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "{1:>0$}", disks_write_total_len, "Written Data")?;

        writeln!(&mut stdout, "")?;

        let mut disks_size_iter = disks_size.into_iter();
        let mut disks_used_iter = disks_used.into_iter();
        let mut disks_used_percentage_iter = disks_used_percentage.into_iter();
        let mut disks_read_iter = disks_read.into_iter();
        let mut disks_read_total_iter = disks_read_total.into_iter();
        let mut disks_write_iter = disks_write.into_iter();
        let mut disks_write_total_iter = disks_write_total.into_iter();

        for disk_with_speed in disks_with_speed.into_iter() {
            let size = disks_size_iter.next().unwrap();

            let used = disks_used_iter.next().unwrap();

            let used_percentage = disks_used_percentage_iter.next().unwrap();

            let read = disks_read_iter.next().unwrap();
            let read_total = disks_read_total_iter.next().unwrap();

            let write = disks_write_iter.next().unwrap();
            let write_total = disks_write_total_iter.next().unwrap();

            stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
            write!(&mut stdout, "{1:<0$}", devices_len_inc, disk_with_speed.disk.device)?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            for _ in 0..(disks_read_len - read.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(disks_read_total_len - read_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(read_total.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(disks_write_len - write.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write.as_bytes())?;

            write!(&mut stdout, "   ")?;

            for _ in 0..(disks_write_total_len - write_total.len()) {
                write!(&mut stdout, " ")?;
            }

            stdout.write_all(write_total.as_bytes())?;

            writeln!(&mut stdout, "")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;

            for _ in 0..devices_len {
                write!(&mut stdout, " ")?;
            }

            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64 / disk_with_speed.disk.size as f64;

            let progress_used = (disk_with_speed.disk.used as f64 * f).floor() as usize;

            stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, "] ")?; // 2

            for _ in 0..(disks_used_len - used.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(used.as_bytes())?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " / ")?; // 3

            for _ in 0..(disks_size_len - size.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(size.as_bytes())?;

            write!(&mut stdout, " (")?; // 2

            for _ in 0..(disks_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.write_all(used_percentage.as_bytes())?;

            write!(&mut stdout, ")")?; // 1

            writeln!(&mut stdout, "")?;

            if mounts {
                stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;

                for point in disk_with_speed.disk.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    writeln!(&mut stdout, "")?;
                }
            }
        }
    }

    output.print(&stdout)?;

    Ok(())
}