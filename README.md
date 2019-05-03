M Prober
====================

[![Build Status](https://travis-ci.org/magiclen/m-prober.svg?branch=master)](https://travis-ci.org/magiclen/m-prober)

This program aims to collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, network interfaces and block devices. It can be used not only as a normal CLI tool, but also a web application with a front-end webpage and useful HTTP APIs.

## Help

```
EXAMPLES:
  mprober hostname                    # Show the hostname
  mprober kernel                      # Show the kernel version
  mprober uptime                      # Show the uptime
  mprober uptime -m                   # Show the uptime and refresh every second
  mprober uptime -p                   # Show the uptime without colors
  mprober uptime -l                   # Show the uptime with darker colors (fitting in with light themes)
  mprober uptime -s                   # Show the uptime in seconds
  mprober time                        # Show the RTC (UTC) date and time
  mprober time -m                     # Show the RTC (UTC) date and time and refresh every second
  mprober time -p                     # Show the RTC (UTC) date and time without colors
  mprober time -l                     # Show the RTC (UTC) date and time with darker colors (fitting in with light themes)
  mprober cpu                         # Show load average and current CPU stats on average
  mprober cpu -m 1000                 # Show load average and CPU stats on average and refresh every 1000 milliseconds
  mprober cpu -p                      # Show load average and current CPU stats on average without colors
  mprober cpu -l                      # Show load average and current CPU stats on average with darker colors (fitting in with light themes)
  mprober cpu -s                      # Show load average and current stats of CPU cores separately
  mprober cpu -i                      # Only show CPU information
  mprober memory                      # Show current memory stats
  mprober memory -m 1000              # Show memory stats and refresh every 1000 milliseconds
  mprober memory -p                   # Show current memory stats without colors
  mprober memory -l                   # Show current memory stats with darker colors (fitting in with light themes)
  mprober memory -u kb                # Show current memory stats in KB
  mprober network                     # Show current network stats
  mprober network -m 1000             # Show network stats and refresh every 1000 milliseconds
  mprober network -p                  # Show current network stats without colors
  mprober network -l                  # Show current network stats with darker colors (fitting in with light themes)
  mprober network -u kb               # Show current network stats in KB
  mprober volume                      # Show current volume stats
  mprober volume -m 1000              # Show current volume stats and refresh every 1000 milliseconds
  mprober volume -p                   # Show current volume stats without colors
  mprober volume -l                   # Show current volume stats without colors
  mprober volume -u kb                # Show current volume stats in KB
  mprober volume -i                   # Only show volume information without I/O rates
  mprober volume --mounts             # Show current volume stats including mount points
  mprober web                         # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 3 seconds.
  mprober web -m 2                    # Start a HTTP service on port 8000 to monitor this computer. The time interval is set to 2 seconds.
  mprober web -p 7777                 # Start a HTTP service on port 7777 to monitor this computer.
  mprober web -a auth_key             # Start a HTTP service on port 8000 to monitor this computer. APIs need to be invoked with an auth key.

USAGE:
    mprober [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    hostname    Shows the hostname
    kernel      Shows the kernel version
    uptime      Shows the uptime
    time        Shows the RTC (UTC) date and time
    cpu         Shows CPU stats
    memory      Shows memory stats
    network     Shows network stats
    volume      Shows volume stats
    web         Starts a HTTP service to monitor this computer
    help        Prints this message or the help of the given subcommand(s)
```

## Requirements

Linux Kernel: 4.4+

## Usage

### Installation / Uninstallation

From [crates.io](https://crates.io/crates/mprober),

```bash
cargo install mprober

# cargo uninstall mprober
```

From [GitHub](https://github.com/magiclen/m-prober),

```bash
(curl -s https://api.github.com/repos/magiclen/m-prober/releases/latest | sed -r -n 's/.*"browser_download_url": *"(.*/mprober_$(uname -m))".*/\1/p' | wget -qi -) && sudo mv mprober_$(uname -m) /usr/local/bin

# sudo rm /usr/local/bin/mprober
```

### CLI

##### Get Hostname


## Crates.io

https://crates.io/crates/m-prober

## Documentation

https://docs.rs/m-prober

## License

[MIT](LICENSE)