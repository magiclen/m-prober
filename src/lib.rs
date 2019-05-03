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
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate lazy_static_include;
extern crate crc_any;

extern crate libc;

extern crate rand;
extern crate base64;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate handlebars;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_simple_authorization;
extern crate rocket_cache_response;
extern crate rocket_json_response;
#[macro_use]
extern crate rocket_include_static_resources;
#[macro_use]
extern crate rocket_include_handlebars;

mod free;
mod cpu_info;
mod load_average;
mod hostname;
mod time;
mod kernel;
mod network;
mod volume;
mod rocket_mounts;

use std::time::Duration;
use std::env;
use std::path::Path;
use std::io::Write;
use std::thread;
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
use volume::{Volume, VolumeWithSpeed};

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

const CLEAR_SCREEN_DATA: [u8; 11] = [0x1b, 0x5b, 0x33, 0x4a, 0x1b, 0x5b, 0x48, 0x1b, 0x5b, 0x32, 0x4a];

static mut LIGHT_MODE: bool = false;
static mut FORCE_PLAIN_MODE: bool = false;

validated_customized_ranged_number!(WebMonitorInterval, u64, 1, 15);

lazy_static! {
    static ref COLOR_DEFAULT: ColorSpec = {
        ColorSpec::new()
    };

    static ref COLOR_LABEL: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(DARK_CYAN_COLOR));
            } else {
                color_spec.set_fg(Some(CYAN_COLOR));
            }
        }

        color_spec
    };

    static ref COLOR_NORMAL_TEXT: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(BLACK_COLOR));
            } else {
                color_spec.set_fg(Some(WHITE_COLOR));
            }
        }

        color_spec
    };

    static ref COLOR_BOLD_TEXT: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(BLACK_COLOR)).set_bold(true);
            } else {
                color_spec.set_fg(Some(WHITE_COLOR)).set_bold(true);
            }
        }

        color_spec
    };

    static ref COLOR_USED: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(WINE_COLOR));
            } else {
                color_spec.set_fg(Some(RED_COLOR));
            }
        }

        color_spec
    };

    static ref COLOR_CACHE: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(ORANGE_COLOR));
            } else {
                color_spec.set_fg(Some(YELLOW_COLOR));
            }
        }

        color_spec
    };

    static ref COLOR_BUFFERS: ColorSpec = {
        let mut color_spec = ColorSpec::new();

        if !unsafe{ FORCE_PLAIN_MODE } {
            if unsafe{ LIGHT_MODE } {
                color_spec.set_fg(Some(DARK_BLUE_COLOR));
            } else {
                color_spec.set_fg(Some(SKY_CYAN_COLOR));
            }
        }

        color_spec
    };
}

// TODO -----Config START-----

#[derive(Debug)]
pub enum Mode {
    HostName,
    Kernel,
    Uptime {
        monitor: bool,
        second: bool,
    },
    Time {
        monitor: bool,
    },
    CPU {
        monitor: Option<Duration>,
        separate: bool,
        information: bool,
    },
    Memory {
        monitor: Option<Duration>,
        unit: Option<ByteUnit>,
    },
    Network {
        monitor: Option<Duration>,
        unit: Option<ByteUnit>,
    },
    Volume {
        monitor: Option<Duration>,
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

const ENV_LIGHT_MODE: &str = "MPROBER_LIGHT";
const ENV_FORCE_PLAIN: &str = "MPROBER_FORCE_PLAIN";

macro_rules! set_color_mode {
    ($sub_matches:ident) => {
        unsafe{
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
                                LIGHT_MODE = env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);
                            }
                        }
                    }
                    None => {
                        if $sub_matches.is_present("LIGHT") {
                            LIGHT_MODE = true;
                        } else {
                            LIGHT_MODE = env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);
                        }
                    }
                }
            }
        }
    };
}

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
            "web                         # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 3 seconds.",
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
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
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
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
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
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
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
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
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
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
                )
                .arg(Arg::with_name("UNIT")
                    .long("unit")
                    .short("u")
                    .help("Forces to use a fixed unit")
                    .takes_value(true)
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("volume").aliases(&["v", "storage", "volumes", "d", "disk", "disks", "blk", "block", "blocks", "mount", "mounts", "ssd", "hdd"])
                .about("Shows volume stats")
                .display_order(7)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows volume stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("LIGHT")
                    .long("light")
                    .short("l")
                    .help("Darker colors")
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
                    .help("Shows only information about volumes without I/O rates")
                )
                .arg(Arg::with_name("MOUNTS")
                    .long("mounts")
                    .aliases(&["mount", "point", "points"])
                    .help("Also shows mount points")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("web").aliases(&["w", "server", "http"])
                .about("Starts a HTTP service to monitor this computer")
                .display_order(8)
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Automatically refreshes every N seconds")
                    .takes_value(true)
                    .value_name("SECONDS")
                    .default_value("3")
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

            let second = sub_matches.is_present("SECOND");

            set_color_mode!(sub_matches);

            Mode::Uptime {
                monitor,
                second,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("time") {
            let monitor = sub_matches.is_present("MONITOR");

            set_color_mode!(sub_matches);

            Mode::Time {
                monitor,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("cpu") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let separate = sub_matches.is_present("SEPARATE");

            let information = sub_matches.is_present("INFORMATION");

            set_color_mode!(sub_matches);

            Mode::CPU {
                monitor,
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

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            set_color_mode!(sub_matches);

            Mode::Memory {
                monitor,
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

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            set_color_mode!(sub_matches);

            Mode::Network {
                monitor,
                unit,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("volume") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGtZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            let information = sub_matches.is_present("INFORMATION");

            let mounts = sub_matches.is_present("MOUNTS");

            set_color_mode!(sub_matches);

            Mode::Volume {
                monitor,
                unit,
                information,
                mounts,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("web") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = WebMonitorInterval::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for SECONDS", monitor))?;

                    Duration::from_secs(monitor.get_number())
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
        Mode::Uptime { monitor, second } => {
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
                    draw_uptime(second, true).map_err(|err| err.to_string())?;

                    thread::sleep(sleep_interval);
                }
            } else {
                draw_uptime(second, false).map_err(|err| err.to_string())?;
            }
        }
        Mode::Time { monitor } => {
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
                    draw_time(true).map_err(|err| err.to_string())?;

                    thread::sleep(sleep_interval);
                }
            } else {
                draw_time(false).map_err(|err| err.to_string())?;
            }
        }
        Mode::CPU { monitor, separate, information } => {
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

                    draw_cpu_info(separate, information, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        if information {
                            thread::sleep(sleep_interval);
                        }

                        draw_cpu_info(separate, information, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_cpu_info(separate, information, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Memory { monitor, unit } => {
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
                        draw_memory(unit, true).map_err(|err| err.to_string())?;

                        thread::sleep(sleep_interval);
                    }
                }
                None => {
                    draw_memory(unit, false).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Network { monitor, unit } => {
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

                    draw_network(unit, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        draw_network(unit, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_network(unit, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Volume { monitor, unit, information, mounts } => {
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

                    draw_volume(unit, information, mounts, Some(Duration::from_millis(DEFAULT_INTERVAL))).map_err(|err| err.to_string())?;

                    let sleep_interval = monitor;

                    loop {
                        if information {
                            thread::sleep(sleep_interval);
                        }

                        draw_volume(unit, information, mounts, Some(sleep_interval)).map_err(|err| err.to_string())?;
                    }
                }
                None => {
                    draw_volume(unit, information, mounts, None).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::Web { monitor, port, auth_key } => {
            rocket_mounts::launch(monitor, port, auth_key);
        }
    }

    Ok(0)
}

fn draw_uptime(second: bool, monitor: bool) -> Result<(), ScannerError> {
    let uptime = time::get_uptime()?;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

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
        let s = time::format_duration(uptime);

        stdout.set_color(&*COLOR_BOLD_TEXT)?;
        stdout.write_all(s.as_bytes())?;
    }

    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
    write!(&mut stdout, ".")?;

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_time(monitor: bool) -> Result<(), ScannerError> {
    let rtc_date_time: RTCDateTime = RTCDateTime::get_rtc_date_time()?;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "RTC Date")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(rtc_date_time.rtc_date.as_bytes())?;

    writeln!(&mut stdout, "")?;

    stdout.set_color(&*COLOR_LABEL)?;
    write!(&mut stdout, "RTC Time")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(&*COLOR_BOLD_TEXT)?;
    stdout.write_all(rtc_date_time.rtc_time.as_bytes())?;

    stdout.set_color(&*COLOR_DEFAULT)?;
    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

// TODO

fn draw_cpu_info(separate: bool, only_information: bool, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
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

    let mut draw_load_average = |cpus: &[CPU]| -> Result<(), ScannerError> {
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
        writeln!(&mut stdout, "")?;

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

        writeln!(&mut stdout, "")?;

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

        writeln!(&mut stdout, "")?;

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

        writeln!(&mut stdout, "")?;
        writeln!(&mut stdout, "")?;

        Ok(())
    };

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            CPUStat::get_all_percentage(false, match monitor {
                Some(monitor) => monitor,
                None => Duration::from_millis(DEFAULT_INTERVAL)
            })?
        };

        let cpus: Vec<CPU> = CPU::get_cpus()?;

        draw_load_average(&cpus)?;

        let mut i = 0;

        let cpus_len_dec = cpus.len() - 1;

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            writeln!(&mut stdout, "")?;

            let mut hz_string: Vec<String> = Vec::with_capacity(cpu.siblings);

            for &cpu_mhz in cpu.cpus_mhz.iter() {
                let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                hz_string.push(format!("{:.2} {}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1]));
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
                    stdout.set_color(&*COLOR_LABEL)?;
                    write!(&mut stdout, "{1:<0$}", d + 4, format!("CPU{}", i))?;

                    stdout.set_color(&*COLOR_BOLD_TEXT)?;
                    write!(&mut stdout, "{1:>0$}", hz_string_len, hz_string)?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
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

                for (i, p) in all_percentage[i..].into_iter().take(cpu.siblings).enumerate() {
                    let percentage_string = percentage_string_iter.next().unwrap();
                    let hz_string = hz_string_iter.next().unwrap();

                    stdout.set_color(&*COLOR_LABEL)?;
                    write!(&mut stdout, "CPU")?; // 3

                    write!(&mut stdout, "{}", i)?;

                    stdout.set_color(&*COLOR_NORMAL_TEXT)?;
                    write!(&mut stdout, " [")?; // 2

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

        let cpus: Vec<CPU> = CPU::get_cpus()?;

        draw_load_average(&cpus)?;

        for cpu in cpus {
            stdout.set_color(&*COLOR_NORMAL_TEXT)?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(&*COLOR_BOLD_TEXT)?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            write!(&mut stdout, " ")?;

            let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

            let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

            write!(&mut stdout, "{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1])?;

            stdout.set_color(&*COLOR_DEFAULT)?;
            writeln!(&mut stdout, "")?;
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
            writeln!(&mut stdout, "")?;
        }
    }

    output.print(&stdout)?;

    Ok(())
}

fn draw_memory(unit: Option<ByteUnit>, monitor: bool) -> Result<(), ScannerError> {
    let free = Free::get_free()?;

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
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

    writeln!(&mut stdout, "")?;

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
    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_network(unit: Option<ByteUnit>, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let networks_with_speed = NetworkWithSpeed::get_networks_with_speed(match monitor {
        Some(monitor) => monitor,
        None => Duration::from_millis(DEFAULT_INTERVAL)
    })?;

    let networks_with_speed_len = networks_with_speed.len();

    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
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

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:<0$}", interface_len_inc, network_with_speed.network.interface)?;

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
        writeln!(&mut stdout, "")?;
    }

    output.print(&stdout)?;

    Ok(())
}

fn draw_volume(unit: Option<ByteUnit>, only_information: bool, mounts: bool, monitor: Option<Duration>) -> Result<(), ScannerError> {
    let output = if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
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
        let volumes = Volume::get_volumes()?;

        let volumes_len = volumes.len();

        debug_assert!(volumes_len > 0);

        let mut volumes_size: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used_percentage: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_read_total: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_write_total: Vec<String> = Vec::with_capacity(volumes_len);

        for volume in volumes.iter() {
            let size = Byte::from_bytes(volume.size as u128);

            let used = Byte::from_bytes(volume.used as u128);

            let used_percentage = format!("{:.2}%", (volume.used * 100) as f64 / volume.size as f64);

            let read_total = Byte::from_bytes(volume.read_bytes as u128);

            let write_total = Byte::from_bytes(volume.write_bytes as u128);

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
        let volumes_used_percentage_len = volumes_used_percentage.iter().map(|used_percentage| used_percentage.len()).max().unwrap();
        let volumes_read_total_len = volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_total_len = volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width - devices_len - 4 - volumes_used_len - 3 - volumes_size_len - 2 - volumes_used_percentage_len - 1;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", devices_len_inc + volumes_read_total_len, "Read Data")?;

        stdout.set_color(&*COLOR_NORMAL_TEXT)?;
        write!(&mut stdout, " | ")?;

        stdout.set_color(&*COLOR_LABEL)?;
        write!(&mut stdout, "{1:>0$}", volumes_write_total_len, "Written Data")?;

        writeln!(&mut stdout, "")?;

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

            writeln!(&mut stdout, "")?;

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
            writeln!(&mut stdout, "")?;

            if mounts {
                stdout.set_color(&*COLOR_NORMAL_TEXT)?;

                for point in volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout, "")?;
                }
            }
        }
    } else {
        let volumes_with_speed = VolumeWithSpeed::get_volumes_with_speed(match monitor {
            Some(monitor) => monitor,
            None => Duration::from_millis(DEFAULT_INTERVAL)
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

        for volume_with_speed in volumes_with_speed.iter() {
            let size = Byte::from_bytes(volume_with_speed.volume.size as u128);

            let used = Byte::from_bytes(volume_with_speed.volume.used as u128);

            let used_percentage = format!("{:.2}%", (volume_with_speed.volume.used * 100) as f64 / volume_with_speed.volume.size as f64);

            let read = Byte::from_unit(volume_with_speed.speed.read, ByteUnit::B).unwrap();
            let read_total = Byte::from_bytes(volume_with_speed.volume.read_bytes as u128);

            let write = Byte::from_unit(volume_with_speed.speed.write, ByteUnit::B).unwrap();
            let write_total = Byte::from_bytes(volume_with_speed.volume.write_bytes as u128);

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

        let devices_len = volumes_with_speed.iter().map(|volume_with_speed| volume_with_speed.volume.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let volumes_size_len = volumes_size.iter().map(|size| size.len()).max().unwrap();
        let volumes_used_len = volumes_used.iter().map(|used| used.len()).max().unwrap();
        let volumes_used_percentage_len = volumes_used_percentage.iter().map(|used_percentage| used_percentage.len()).max().unwrap();
        let volumes_read_len = volumes_read.iter().map(|read| read.len()).max().unwrap().max(12);
        let volumes_read_total_len = volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_len = volumes_write.iter().map(|write| write.len()).max().unwrap().max(12);
        let volumes_write_total_len = volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width - devices_len - 4 - volumes_used_len - 3 - volumes_size_len - 2 - volumes_used_percentage_len - 1;

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

        writeln!(&mut stdout, "")?;

        let mut volumes_size_iter = volumes_size.into_iter();
        let mut volumes_used_iter = volumes_used.into_iter();
        let mut volumes_used_percentage_iter = volumes_used_percentage.into_iter();
        let mut volumes_read_iter = volumes_read.into_iter();
        let mut volumes_read_total_iter = volumes_read_total.into_iter();
        let mut volumes_write_iter = volumes_write.into_iter();
        let mut volumes_write_total_iter = volumes_write_total.into_iter();

        for volume_with_speed in volumes_with_speed.into_iter() {
            let size = volumes_size_iter.next().unwrap();

            let used = volumes_used_iter.next().unwrap();

            let used_percentage = volumes_used_percentage_iter.next().unwrap();

            let read = volumes_read_iter.next().unwrap();
            let read_total = volumes_read_total_iter.next().unwrap();

            let write = volumes_write_iter.next().unwrap();
            let write_total = volumes_write_total_iter.next().unwrap();

            stdout.set_color(&*COLOR_LABEL)?;
            write!(&mut stdout, "{1:<0$}", devices_len_inc, volume_with_speed.volume.device)?;

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

            writeln!(&mut stdout, "")?;

            stdout.set_color(&*COLOR_NORMAL_TEXT)?;

            for _ in 0..devices_len {
                write!(&mut stdout, " ")?;
            }

            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64 / volume_with_speed.volume.size as f64;

            let progress_used = (volume_with_speed.volume.used as f64 * f).floor() as usize;

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
            writeln!(&mut stdout, "")?;

            if mounts {
                stdout.set_color(&*COLOR_NORMAL_TEXT)?;

                for point in volume_with_speed.volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ")?;
                    }

                    stdout.write_all(point.as_bytes())?;

                    stdout.set_color(&*COLOR_DEFAULT)?;
                    writeln!(&mut stdout, "")?;
                }
            }
        }
    }

    output.print(&stdout)?;

    Ok(())
}