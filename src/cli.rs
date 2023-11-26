use std::{
    net::{AddrParseError, IpAddr},
    num::ParseIntError,
    str::FromStr,
    time::Duration,
};

use byte_unit::{Unit, UnitParseError};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use concat_with::concat_line;
use regex::Regex;
use terminal_size::terminal_size;

const APP_NAME: &str = "M Prober";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "M Prober is a free and simple probe utility for Linux.\n\nEXAMPLES:\n",
    concat_line!(prefix "mprober ",
        "hostname                      # Show the hostname",
        "kernel                        # Show the kernel version",
        "uptime                        # Show the uptime",
        "uptime -m                     # Show the uptime and refresh every second",
        "uptime -p                     # Show the uptime without colors",
        "uptime -l                     # Show the uptime with darker colors (fitting in with light themes)",
        "uptime -s                     # Show the uptime in seconds",
        "time                          # Show the RTC (UTC) date and time",
        "time -m                       # Show the RTC (UTC) date and time and refresh every second",
        "time -p                       # Show the RTC (UTC) date and time without colors",
        "time -l                       # Show the RTC (UTC) date and time with darker colors (fitting in with light themes)",
        "cpu                           # Show load average and current CPU stats on average",
        "cpu -m 1000                   # Show load average and CPU stats on average and refresh every 1000 milliseconds",
        "cpu -p                        # Show load average and current CPU stats on average without colors",
        "cpu -l                        # Show load average and current CPU stats on average with darker colors (fitting in with light themes)",
        "cpu -s                        # Show load average and current stats of CPU cores separately",
        "cpu -i                        # Only show CPU information",
        "memory                        # Show current memory stats",
        "memory -m 1000                # Show memory stats and refresh every 1000 milliseconds",
        "memory -p                     # Show current memory stats without colors",
        "memory -l                     # Show current memory stats with darker colors (fitting in with light themes)",
        "memory -u kb                  # Show current memory stats in KB",
        "network                       # Show current network stats",
        "network -m 1000               # Show network stats and refresh every 1000 milliseconds",
        "network -p                    # Show current network stats without colors",
        "network -l                    # Show current network stats with darker colors (fitting in with light themes)",
        "network -u kb                 # Show current network stats in KB",
        "volume                        # Show current volume stats",
        "volume -m 1000                # Show current volume stats and refresh every 1000 milliseconds",
        "volume -p                     # Show current volume stats without colors",
        "volume -l                     # Show current volume stats without colors",
        "volume -u kb                  # Show current volume stats in KB",
        "volume -i                     # Only show volume information without I/O rates",
        "volume --mounts               # Show current volume stats including mount points",
        "process                       # Show a snapshot of the current processes",
        "process -m 1000               # Show a snapshot of the current processes and refresh every 1000 milliseconds",
        "process -p                    # Show a snapshot of the current processes without colors",
        "process -l                    # Show a snapshot of the current processes with darker colors (fitting in with light themes)",
        "process -i                    # Show a snapshot of the current processes but not including CPU usage",
        "process -u kb                 # Show a snapshot of the current processes. Information about memory size is in KB",
        "process --truncate 10         # Show a snapshot of the current processes with a specific truncation length to truncate user, group, program's names",
        "process --top 10              # Show a snapshot of current top-10 (ordered by CPU and memory usage) processes",
        "process -t                    # Show a snapshot of the current processes with the start time of each process",
        "process --pid-filter 3456     # Show a snapshot of the current processes which are related to a specific PID",
        "process --user-filter user1   # Show a snapshot of the current processes which are related to a specific user",
        "process --group-filter gp1    # Show a snapshot of the current processes which are related to a specific group",
        "process --tty-filter tty      # Show a snapshot of the current processes which are related to specific tty names matched by a regex",
        "process --program-filter ab   # Show a snapshot of the current processes which are related to specific program names or commands matched by a regex",
        "web                           # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 3 seconds",
        "web -m 2                      # Start a HTTP service on port 8000 to monitor this computer. The time interval is set to 2 seconds",
        "web -p 7777                   # Start a HTTP service on port 7777 to monitor this computer",
        "web --addr 127.0.0.1          # Start a HTTP service on 127.0.0.1:8000 to monitor this computer",
        "web -a auth_key               # Start a HTTP service on port 8000 to monitor this computer. APIs need to be invoked with an auth key",
        "web --only-api                # Start a HTTP service on port 8000 to serve only HTTP APIs",
        "benchmark                     # Run benchmarks",
        "benchmark --disable-cpu       # Run benchmarks except for benchmarking CPU",
        "benchmark --enable-memory     # Benchmark the memory",
    )
);

#[derive(Debug, Parser)]
#[command(name = APP_NAME)]
#[command(term_width = terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))]
#[command(version = CARGO_PKG_VERSION)]
#[command(author = CARGO_PKG_AUTHORS)]
#[command(after_help = AFTER_HELP)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: CLICommands,
}

#[derive(Debug, Subcommand)]
pub enum CLICommands {
    #[command(aliases = ["h", "host", "name", "servername"])]
    #[command(about = "Show the hostname")]
    #[command(after_help = AFTER_HELP)]
    Hostname,
    #[command(aliases = ["k", "l", "linux"])]
    #[command(about = "Show the kernel version")]
    #[command(after_help = AFTER_HELP)]
    Kernel,
    #[command(aliases = ["u", "up", "utime", "ut"])]
    #[command(about = "Show the uptime")]
    #[command(after_help = AFTER_HELP)]
    Uptime {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:   bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:   bool,
        #[arg(short, long)]
        #[arg(help = "Show the uptime and refresh every second")]
        monitor: bool,
        #[arg(short, long)]
        #[arg(help = "Show the uptime in seconds")]
        second:  bool,
    },
    #[command(aliases = ["t", "systime", "stime", "st", "utc", "utctime", "rtc", "rtctime", "date"])]
    #[command(about = "Show the RTC (UTC) date and time")]
    #[command(after_help = AFTER_HELP)]
    Time {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:   bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:   bool,
        #[arg(short, long)]
        #[arg(help = "Show the RTC (UTC) date and time, and refresh every second")]
        monitor: bool,
    },
    #[command(aliases = ["c", "cpus", "core", "cores", "load", "processor", "processors"])]
    #[command(about = "Show CPU stats")]
    #[command(after_help = AFTER_HELP)]
    Cpu {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:            bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:            bool,
        #[arg(short, long, value_name = "MILLI_SECONDS")]
        #[arg(num_args = 0..=1, default_missing_value = "1000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Show CPU stats and refresh every N milliseconds")]
        monitor:          Option<Duration>,
        #[arg(short, long)]
        #[arg(help = "Separates each CPU")]
        separate:         bool,
        #[arg(short = 'i', long)]
        #[arg(help = "Show only information about CPUs")]
        only_information: bool,
    },
    #[command(aliases = [ "m", "mem", "f", "free", "memories", "swap", "ram", "dram", "ddr", "cache", "buffer", "buffers", "buf", "buff"])]
    #[command(about = "Show memory stats")]
    #[command(after_help = AFTER_HELP)]
    Memory {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:   bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:   bool,
        #[arg(short, long, value_name = "MILLI_SECONDS")]
        #[arg(num_args = 0..=1, default_missing_value = "1000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Show memory stats and refresh every N milliseconds")]
        monitor: Option<Duration>,
        #[arg(short, long)]
        #[arg(value_parser = parse_unit)]
        #[arg(help = "Forces to use a fixed unit")]
        unit:    Option<Unit>,
    },
    #[command(aliases = ["n", "net", "networks", "bandwidth", "traffic"])]
    #[command(about = "Show network stats")]
    #[command(after_help = AFTER_HELP)]
    Network {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:   bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:   bool,
        #[arg(short, long, value_name = "MILLI_SECONDS")]
        #[arg(num_args = 0..=1, default_missing_value = "1000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Show network stats and refresh every N milliseconds")]
        monitor: Option<Duration>,
        #[arg(short, long)]
        #[arg(value_parser = parse_unit)]
        #[arg(help = "Forces to use a fixed unit")]
        unit:    Option<Unit>,
    },
    #[command(aliases = ["v", "storage", "volumes", "d", "disk", "disks", "blk", "block", "blocks", "mount", "mounts", "ssd", "hdd"])]
    #[command(about = "Show volume stats")]
    #[command(after_help = AFTER_HELP)]
    Volume {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:            bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:            bool,
        #[arg(short, long, value_name = "MILLI_SECONDS")]
        #[arg(num_args = 0..=1, default_missing_value = "1000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Show volume stats and refresh every N milliseconds")]
        monitor:          Option<Duration>,
        #[arg(short, long)]
        #[arg(value_parser = parse_unit)]
        #[arg(help = "Forces to use a fixed unit")]
        unit:             Option<Unit>,
        #[arg(short = 'i', long)]
        #[arg(help = "Show only information about volumes without I/O rates")]
        only_information: bool,
        #[arg(long, aliases = ["mount", "point", "points"])]
        #[arg(help = "Also shows mount points")]
        mounts:           bool,
    },
    #[command(aliases = ["p", "ps"])]
    #[command(about = "Show process stats")]
    #[command(after_help = AFTER_HELP)]
    Process {
        #[arg(short, long)]
        #[arg(help = "No colors")]
        plain:            bool,
        #[arg(short, long)]
        #[arg(help = "Darker colors")]
        light:            bool,
        #[arg(short, long, value_name = "MILLI_SECONDS")]
        #[arg(num_args = 0..=1, default_missing_value = "1000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Show process stats and refresh every N milliseconds")]
        monitor:          Option<Duration>,
        #[arg(short, long)]
        #[arg(value_parser = parse_unit)]
        #[arg(help = "Forces to use a fixed unit")]
        unit:             Option<Unit>,
        #[arg(short = 'i', long)]
        #[arg(help = "Show only information about processes without CPU usage")]
        only_information: bool,
        #[arg(long, value_name = "MAX_NUMBER_OF_PROCESSES")]
        #[arg(help = "Sets the max number of processes shown on the screen")]
        top:              Option<usize>,
        #[arg(long, value_name = "LENGTH")]
        #[arg(default_value = "7")]
        #[arg(help = "Truncate the user name, the group name and the program name of processes. \
                      Set '0' to disable")]
        truncate:         usize,
        #[arg(short = 't', long)]
        #[arg(help = "Show when the progresses start")]
        start_time:       bool,
        #[arg(long, alias = "filter-user", value_name = "USER_NAME")]
        #[arg(help = "Show only processes which are related to a specific user")]
        user_filter:      Option<String>,
        #[arg(long, alias = "filter-group", value_name = "GROUP_NAME")]
        #[arg(help = "Show only processes which are related to a specific group")]
        group_filter:     Option<String>,
        #[arg(long, alias = "filter-program", value_name = "REGEX")]
        #[arg(value_parser = parse_regex)]
        #[arg(help = "Show only processes which are related to specific programs or commands \
                      matched by a regex")]
        program_filter:   Option<Regex>,
        #[arg(long, alias = "filter-tty", value_name = "REGEX")]
        #[arg(value_parser = parse_regex)]
        #[arg(help = "Show only processes which are run on specific TTY/PTS matched by a regex")]
        tty_filter:       Option<Regex>,
        #[arg(long, visible_alias = "pid", alias = "filter-pid", value_name = "PID")]
        #[arg(help = "Show only processes which are related to a specific PID")]
        pid_filter:       Option<u32>,
    },
    #[command(aliases = ["w", "server", "http"])]
    #[command(about = "Start a HTTP service to monitor this computer")]
    #[command(after_help = AFTER_HELP)]
    Web {
        #[arg(short, long, value_name = "SECONDS")]
        #[arg(default_value = "3")]
        #[arg(value_parser = parse_duration_sec)]
        #[arg(help = "Automatically refresh every N seconds")]
        monitor:     Duration,
        #[arg(long, visible_alias = "addr")]
        #[cfg_attr(debug_assertions, arg(default_value = "127.0.0.1"))]
        #[cfg_attr(not(debug_assertions), arg(default_value = "0.0.0.0"))]
        #[arg(value_parser = parse_ip_addr)]
        #[arg(help = "Assign the address that M Prober binds")]
        address:     IpAddr,
        #[arg(short = 'p', long, visible_alias = "port")]
        #[arg(default_value = "8000")]
        #[arg(help = "Assign a TCP port for the HTTP service")]
        listen_port: u16,
        #[arg(short, long)]
        #[arg(help = "Assign an auth key")]
        auth_key:    Option<String>,
        #[arg(long, aliases = ["only-apis"])]
        #[arg(help = "Disable the web page")]
        only_api:    bool,
    },
    #[command(aliases = ["b", "bench", "performance"])]
    #[command(about = "Run benchmarks to measure the performance of this environment")]
    #[command(after_help = AFTER_HELP)]
    Benchmark {
        #[arg(long, value_name = "MILLI_SECONDS")]
        #[arg(default_value = "3000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Assign a duration for warming up")]
        warming_up_duration: Duration,
        #[arg(long, value_name = "MILLI_SECONDS")]
        #[arg(default_value = "5000")]
        #[arg(value_parser = parse_duration)]
        #[arg(help = "Assign a duration for each benchmarking")]
        benchmark_duration:  Duration,
        #[arg(short, long)]
        #[arg(help = "Show more information in stderr")]
        verbose:             bool,
        #[arg(long, aliases = ["disabled-cpu", "disable-cpus", "disabled-cpus"])]
        #[arg(conflicts_with = "enable_cpu")]
        #[arg(help = "Not to benchmark CPUs")]
        disable_cpu:         bool,
        #[arg(long, aliases = ["enabled-cpu", "enable-cpus", "enabled-cpus"])]
        #[arg(conflicts_with = "disable_cpu")]
        #[arg(help = "Allow to benchmark CPUs (disables others by default)")]
        enable_cpu:          bool,
        #[arg(long, aliases = ["disabled-memory"])]
        #[arg(conflicts_with = "enable_memory")]
        #[arg(help = "Not to benchmark memory")]
        disable_memory:      bool,
        #[arg(long, aliases = ["enabled-memory"])]
        #[arg(conflicts_with = "disable_memory")]
        #[arg(help = "Allow to benchmark memory (disables others by default)")]
        enable_memory:       bool,
        #[arg(long, aliases = ["disabled-volume", "disable-volumes", "disabled-volumes"])]
        #[arg(conflicts_with = "enable_volume")]
        #[arg(help = "Not to benchmark volumes")]
        disable_volume:      bool,
        #[arg(long, aliases = ["enabled-volume", "enable-volumes", "enabled-volumes"])]
        #[arg(conflicts_with = "disable_volume")]
        #[arg(help = "Allow to benchmark volumes (disables others by default)")]
        enable_volume:       bool,
    },
}

#[inline]
fn parse_duration(arg: &str) -> Result<Duration, ParseIntError> {
    Ok(Duration::from_millis(arg.parse()?))
}

#[inline]
fn parse_duration_sec(arg: &str) -> Result<Duration, ParseIntError> {
    Ok(Duration::from_secs(arg.parse()?))
}

#[inline]
fn parse_unit(arg: &str) -> Result<Unit, UnitParseError> {
    Unit::from_str(arg)
}

#[inline]
fn parse_regex(arg: &str) -> Result<Regex, regex::Error> {
    Regex::new(arg)
}

#[inline]
fn parse_ip_addr(arg: &str) -> Result<IpAddr, AddrParseError> {
    IpAddr::from_str(arg)
}

pub fn get_args() -> CLIArgs {
    let args = CLIArgs::command();

    let about = format!("{APP_NAME} {CARGO_PKG_VERSION}\n{CARGO_PKG_AUTHORS}\n{APP_ABOUT}");

    let args = args.about(about);

    let matches = args.get_matches();

    match CLIArgs::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(err) => {
            err.exit();
        },
    }
}
