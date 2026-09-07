M Prober
====================

[![CI](https://github.com/magiclen/m-prober/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/m-prober/actions/workflows/ci.yml)

This program collects Linux system information: hostname, kernel version, uptime, RTC time, load average, CPU, memory, PSI pressure, cgroup limits, network interfaces, block devices and processes. It is meant as a probe for a VPS, a cloud instance or a remote virtual machine, and works both as a CLI tool and as a web application with a front-end page and HTTP APIs.

## Help

```
M Prober 0.12.0
Magic Len <len@magiclen.org>
M Prober is a free and simple probe utility for Linux.

EXAMPLES:
mprober hostname                      # Show the hostname
mprober kernel                        # Show the kernel version
mprober uptime                        # Show the uptime
mprober uptime -m                     # Show the uptime and refresh every second
mprober uptime -p                     # Show the uptime without colors
mprober uptime -l                     # Show the uptime with darker colors (fitting in with light themes)
mprober uptime -s                     # Show the uptime in seconds
mprober time                          # Show the RTC (UTC) date and time
mprober time -m                       # Show the RTC (UTC) date and time and refresh every second
mprober time -p                       # Show the RTC (UTC) date and time without colors
mprober time -l                       # Show the RTC (UTC) date and time with darker colors (fitting in with light themes)
mprober cpu                           # Show load average and current CPU stats on average
mprober cpu -m 1000                   # Show load average and CPU stats on average and refresh every 1000 milliseconds
mprober cpu -p                        # Show load average and current CPU stats on average without colors
mprober cpu -l                        # Show load average and current CPU stats on average with darker colors (fitting in with light themes)
mprober cpu -s                        # Show load average and current stats of CPU cores separately
mprober cpu -i                        # Only show CPU information
mprober memory                        # Show current memory stats
mprober memory -m 1000                # Show memory stats and refresh every 1000 milliseconds
mprober memory -p                     # Show current memory stats without colors
mprober memory -l                     # Show current memory stats with darker colors (fitting in with light themes)
mprober memory -u kb                  # Show current memory stats in KB
mprober network                       # Show current network stats
mprober network -m 1000               # Show network stats and refresh every 1000 milliseconds
mprober network -p                    # Show current network stats without colors
mprober network -l                    # Show current network stats with darker colors (fitting in with light themes)
mprober network -u kb                 # Show current network stats in KB
mprober volume                        # Show current volume stats
mprober volume -m 1000                # Show current volume stats and refresh every 1000 milliseconds
mprober volume -p                     # Show current volume stats without colors
mprober volume -l                     # Show current volume stats without colors
mprober volume -u kb                  # Show current volume stats in KB
mprober volume -i                     # Only show volume information without I/O rates
mprober volume --mounts               # Show current volume stats including mount points
mprober pressure                      # Show PSI, which tells resource shortage apart from a busy but healthy system
mprober pressure -m 1000              # Show PSI and refresh every 1000 milliseconds
mprober cgroup                        # Show the CPU, memory and PID limits of the container or VM this runs in
mprober cgroup -m 1000                # Show the cgroup stats and refresh every 1000 milliseconds
mprober cgroup -u kb                  # Show the cgroup stats in KB
mprober process                       # Show a snapshot of the current processes
mprober process -m 1000               # Show a snapshot of the current processes and refresh every 1000 milliseconds
mprober process -p                    # Show a snapshot of the current processes without colors
mprober process -l                    # Show a snapshot of the current processes with darker colors (fitting in with light themes)
mprober process -i                    # Show a snapshot of the current processes but not including CPU usage
mprober process -u kb                 # Show a snapshot of the current processes. Information about memory size is in KB
mprober process --truncate 10         # Show a snapshot of the current processes with a specific truncation length to truncate user, group, program's names
mprober process --top 10              # Show a snapshot of current top-10 (ordered by CPU and memory usage) processes
mprober process -t                    # Show a snapshot of the current processes with the start time of each process
mprober process --pid-filter 3456     # Show a snapshot of the current processes which are related to a specific PID
mprober process --user-filter user1   # Show a snapshot of the current processes which are related to a specific user
mprober process --group-filter gp1    # Show a snapshot of the current processes which are related to a specific group
mprober process --tty-filter tty      # Show a snapshot of the current processes which are related to specific tty names matched by a regex
mprober process --program-filter ab   # Show a snapshot of the current processes which are related to specific program names or commands matched by a regex
mprober web                           # Start a HTTP service on port 8000 to monitor this computer. The default time interval is 3 seconds
mprober web -m 2                      # Start a HTTP service on port 8000 to monitor this computer. The time interval is set to 2 seconds
mprober web -p 7777                   # Start a HTTP service on port 7777 to monitor this computer
mprober web --addr 127.0.0.1          # Start a HTTP service on 127.0.0.1:8000 to monitor this computer
mprober web -a auth_key               # Start a HTTP service on port 8000 to monitor this computer. APIs need to be invoked with an auth key
mprober web --only-api                # Start a HTTP service on port 8000 to serve only HTTP APIs
mprober benchmark                     # Run benchmarks
mprober benchmark --disable-cpu       # Run benchmarks except for benchmarking CPU
mprober benchmark --enable-memory     # Benchmark the memory

Usage: mprober <COMMAND>

Commands:
  hostname   Show the hostname
  kernel     Show the kernel version
  uptime     Show the uptime
  time       Show the RTC (UTC) date and time
  cpu        Show CPU stats
  memory     Show memory stats
  network    Show network stats
  volume     Show volume stats
  pressure   Show PSI (Pressure Stall Information)
  cgroup     Show the limits and usage of the cgroup this program runs in
  process    Show process stats
  web        Start a HTTP service to monitor this computer
  benchmark  Run benchmarks to measure the performance of this environment
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Enjoy it! https://magiclen.org
```

## Requirements

* Linux kernel 5.10 or later.
* `pressure` needs a kernel built with `CONFIG_PSI` which was not booted with `psi=0`.
* `cgroup` needs cgroup v2, which is what every current distribution mounts.

Both subcommands say so and exit instead of failing when the machine does not provide the data, and their HTTP APIs answer `404` there.

## Usage

### Installation / Uninstallation

From [crates.io](https://crates.io/crates/mprober),

```bash
cargo install mprober

# cargo uninstall mprober
```

From [GitHub](https://github.com/magiclen/m-prober) (x86 and x86_64),

```bash
(curl -s https://api.github.com/repos/magiclen/m-prober/releases/latest | sed -r -n 's/.*"browser_download_url": *"(.*\/mprober_'$(uname -m)')".*/\1/p' | wget -i -) && sudo mv mprober_$(uname -m) /usr/local/bin/mprober && sudo chmod +x /usr/local/bin/mprober

# sudo rm /usr/local/bin/mprober
```

### CLI

##### Get Hostname

```bash
mprober hostname
```

In addition to `hostname`, `h`, `host`, `name`, and `servername` are also acceptable.

![hostname.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/hostname.png)

##### Get Kernel Version

```bash
mprober kernel
```

In addition to `kernel`, `k`, `l`, and `linux` are also acceptable.

![kernel.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/kernel.png)

##### Get System Uptime

```bash
mprober uptime
```

In addition to `uptime`, `u`, `up`, `utime`, and `ut` are also acceptable.

![uptime.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/uptime.png)

##### Get RTC Time

```bash
mprober time
```

In addition to `time`, `t`, `systime`, `stime`, `st`, `utc`, `utctime`, `rtc`, `rtctime`, and `date` are also acceptable.

![time.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/time.png)

##### Show CPU Stats

```bash
mprober cpu
```

In addition to `cpu`, `c`, `cpus`, `core`, `cores`, `load`, `processor`, and `processors` are also acceptable.

![cpu.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/cpu.png)

##### Show Memory Stats

```bash
mprober memory
```

In addition to `memory`, `m`, `mem`, `f`,`free`, `memories`, `swap`, `ram`, `dram`, `ddr`, `cache`, `buffer`, `buffers`, `buf`, and `buff` are also acceptable.

![memory.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/memory.png)

##### Show Network Stats

```bash
mprober network
```

In addition to `network`, `n`, `net`, `networks`,`bandwidth`, and `traffic` are also acceptable.

![network.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/network.png)

##### Show Volume Stats

```bash
mprober volume
```

In addition to `volume`, `v`, `storage`, `volumes`, `d`, `disk`, `disks`, `blk`, `block`, `blocks`, `mount`, `mounts`, `ssd`, and `hdd` are also acceptable.

![volume.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/volume.png)

##### Show Pressure (PSI)

```bash
mprober pressure
```

In addition to `pressure`, `psi`, `stall`, and `pressures` are also acceptable.

PSI is the share of time tasks spent stalled waiting for a resource. Unlike the load average it tells a system which is merely busy apart from one which is actually short of CPU, memory or I/O. `some` is the time at least one task was stalled, `full` the time every non-idle task was.

##### Show cgroup Limits

```bash
mprober cgroup
```

In addition to `cgroup`, `g`, `container`, `limit`, `limits`, and `cgroups` are also acceptable.

This shows the CPU quota, the memory limit and the PID limit that the container or the cloud instance actually caps this machine at, which is often lower than what `cpu` and `memory` report for the host.

#### Color Mode

Environment variables, `MPROBER_LIGHT` and `MPROBER_FORCE_PLAIN` can be used to control the output colors.

![colors.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/colors.png)

#### Benchmark

To benchmark the performance of CPU, memory and volumes,

```bash
mprober benchmark
```

In addition to `benchmark`, `b`, `bench`, and `performance` are also acceptable.

Adding the `--disable-xxx` or `--enable-xxx` flags can control what benchmarks you want to run.

![web.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/benchmark.png)

### Web (HTTP)

#### Launching the Server

```bash
mprober web
```

In addition to `web`, `w`, `server`, and `http` are also acceptable.

Once you start the server, you can open [`http://0.0.0.0:8000`](http://0.0.0.0:8000) via a web browser such as Firefox or Chrome.

![web.png](https://raw.githubusercontent.com/magiclen/m-prober/master/doc-images/web.png)

To change the listening port, use the `-p <PORT>` option. To change the detecting time interval, use the `-m <SECONDS>` option. To bind somewhere other than `0.0.0.0`, use `--addr <ADDRESS>`.

One background sampler serves every client, so opening the page in several tabs still costs one sampling round per interval, and the sampler stops entirely while nobody is watching.

#### HTTP APIs

Every response is the JSON that [`mprober-lib`](https://crates.io/crates/mprober-lib) serializes, so the field names are those of its types. A `std::time::Duration` is an object rather than a number:

```json
{ "secs": 7942, "nanos": 390000000 }
```

An endpoint answers `404` with `{"error": "..."}` when the kernel does not provide the data at all, e.g. `/api/pressure` on a kernel without `CONFIG_PSI`. It answers `500` with the same shape when a probe fails.

##### Instant readings

These read `/proc` and `/sys` once and answer right away.

| Endpoint | Content |
| --- | --- |
| *GET* `/api/hostname` | The hostname, as a JSON string. |
| *GET* `/api/kernel` | The kernel version, as a JSON string. |
| *GET* `/api/uptime` | `total_uptime` and `all_cpu_idle_time`. |
| *GET* `/api/time` | The RTC (UTC) date and time, as an ISO 8601 string. |
| *GET* `/api/cpu` | `load_average` and `cpus`. |
| *GET* `/api/memory` | `mem` and `swap`, in bytes. |
| *GET* `/api/network` | Every interface with its counters. |
| *GET* `/api/volume` | Every mounted volume with its size, usage and I/O counters. |
| *GET* `/api/pressure` | The PSI of `cpu`, `memory` and `io`. |
| *GET* `/api/cgroup` | The full cgroup report: `cpu`, `cpuset`, `memory`, `memory_stat`, `memory_events`, `pids` and `io`. |
| *GET* `/api/config` | The version and the detect interval. Reachable without an auth key. |

For example, *GET* `/api/memory`:

```json
{
    "mem": {
        "total": 66645028864,
        "used": 21888958464,
        "free": 2135752704,
        "shared": 689647616,
        "buffers": 6873088,
        "cache": 44048076800,
        "available": 44756070400
    },
    "swap": {
        "total": 8192520192,
        "used": 0,
        "free": 8192520192,
        "cache": 0
    }
}
```

##### Sampled readings

A rate can only be measured over a period, so these serve the latest snapshot of the shared sampler. The first request after an idle period waits for one detect interval; the rest are answered immediately.

| Endpoint | Content |
| --- | --- |
| *GET* `/api/cpu-detect` | `load_average`, `cpus` and `cpus_stat`. |
| *GET* `/api/network-detect` | Every interface with its counters and its `speed`. |
| *GET* `/api/volume-detect` | Every volume with its counters and its `speed`. |
| *GET* `/api/all` | One full snapshot, which is everything above in a single response. |

`cpus_stat` is the CPU utilization as a fraction from `0` to `1`. Its first entry is the average over every CPU and the rest are the individual ones.

*GET* `/api/all` answers with:

```json
{
    "hostname": "magiclen-linux",
    "kernel": "6.17.0-40-generic",
    "uptime": { "total_uptime": { "secs": 7942, "nanos": 390000000 }, "all_cpu_idle_time": { "secs": 187517, "nanos": 980000000 } },
    "rtc_time": "2026-09-07T13:01:46",
    "load_average": { "one": 0.55, "five": 0.62, "fifteen": 0.7 },
    "cpus": [ { "physical_id": 0, "model_name": "Intel(R) Core(TM) Ultra 9 285K", "cpus_mhz": [800.0], "siblings": 24, "cpu_cores": 24 } ],
    "cpus_stat": [0.0375, 0.14, 0.01],
    "memory": { "mem": {}, "swap": {} },
    "network": [ { "interface": "lo", "stat": {}, "speed": { "receive": 0.0, "transmit": 0.0, "receive_packets": 0.0, "transmit_packets": 0.0 } } ],
    "volumes": [ { "device": "nvme0n1p1", "stat": {}, "size": 97033216, "used": 6399488, "fs_type": "vfat", "points": ["/boot/efi"], "speed": {} } ],
    "pressure": { "cpu": {}, "memory": {}, "io": {} },
    "cgroup": { "path": "/sys/fs/cgroup/...", "cpu": {}, "memory": {}, "pids": {} }
}
```

`pressure` is `null` when the kernel provides no PSI, and `cgroup` is `null` when the program does not run under cgroup v2.

##### *GET* `/api/all/stream`

The same snapshot, pushed over [Server-Sent Events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events) once per detect interval. This is what the web page uses, so that a browser does not have to poll.

```bash
curl -N http://127.0.0.1:8000/api/all/stream
```

##### Authorization

If you expose these APIs to the Internet, add the `-a <AUTH_KEY>` option so that not just anyone can invoke them.

A request may then carry the key in an `Authorization` header:

```bash
curl -H 'Authorization: <AUTH_KEY>' http://127.0.0.1:8000/api/all
```

`EventSource` cannot set headers, so the web page signs in instead and gets a session cookie:

| Endpoint | Content |
| --- | --- |
| *GET* `/api/auth` | `{"required": bool, "authenticated": bool}`. Reachable without an auth key. |
| *POST* `/api/auth` | Takes `{"auth_key": "..."}`. Answers `204` and sets an `HttpOnly` cookie, or `401`. |
| *POST* `/api/logout` | Clears the cookie. |

The cookie holds a random token this process generates at startup, so the key itself never leaves the server. The `Secure` attribute is set when a reverse proxy reports `X-Forwarded-Proto: https`.

Also, you may want to disable the web page. Just add a `--only-api` flag.

#### Developing the Web UI

The page lives in [`web-ui`](web-ui) and is built with Vite, React and Mantine. Its build output is committed under [`front-end`](front-end) and embedded into the executable, so `cargo install mprober` needs no Node. See [`web-ui/README.md`](web-ui/README.md).

## TODO

1. Process snapshot (HTTP API, web page)
1. Sensors and batteries (`hwmon`, `power_supply`)
1. Sockets and routes
1. Database Detection
1. Benchmark (networks)

## License

[MIT](LICENSE)