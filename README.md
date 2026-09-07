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

Every subcommand accepts a few shared flags:

| Flag | Effect |
| --- | --- |
| `-m`, `--monitor` | Redraw on an interval instead of printing once. Press `q` to leave. |
| `-p`, `--plain` | No colors, and the bars are drawn with `\|`, `$` and `#` so they stay readable. |
| `-l`, `--light` | Darker colors, which fit a light terminal theme. |
| `-u`, `--unit` | Force a fixed unit, e.g. `-u kb`, instead of picking one per value. |

`MPROBER_FORCE_PLAIN` and `MPROBER_LIGHT` set `--plain` and `--light` for every run, which is useful when the output is piped or when the terminal has a light theme. Set either to anything other than `0` to enable it.

The samples below are the plain output, so they are what you get when the color escapes are stripped.

#### Get Hostname

```bash
mprober hostname
```

```
magiclen-linux
```

In addition to `hostname`, `h`, `host`, `name`, and `servername` are also acceptable.

#### Get Kernel Version

```bash
mprober kernel
```

```
6.17.0-40-generic
```

In addition to `kernel`, `k`, `l`, and `linux` are also acceptable.

#### Get System Uptime

```bash
mprober uptime
```

```
This computer has been up for 2 hours, 54 minutes, and 41 seconds.
```

Add `-s` to get the number of seconds instead.

In addition to `uptime`, `u`, `up`, `utime`, and `ut` are also acceptable.

#### Get RTC Time

```bash
mprober time
```

```
RTC Date 2026-09-07
RTC Time 13:43:28
```

In addition to `time`, `t`, `systime`, `stime`, `st`, `utc`, `utctime`, `rtc`, `rtctime`, and `date` are also acceptable.

#### Show CPU Stats

```bash
mprober cpu
```

```
There are 24 logical CPU cores.
one     [|||                                                      ] 1.42 (5.92%)
five    [||                                                       ] 0.86 (3.58%)
fifteen [|                                                        ] 0.48 (2.00%)

Intel(R) Core(TM) Ultra 9 285K 24C/24T 1.67GHz
CPU [||||||                                                              ] 9.51%
```

Add `-s` to get a bar and a frequency per core instead of the average, one row per core:

```
Intel(R) Core(TM) Ultra 9 285K 24C/24T
CPU0  [|||||||||||||||||||||||||||||||||||||||||||||||||||] 100.00% (  5.50 GHz)
CPU1  [                                                   ]   0.00% (  4.96 GHz)
CPU2  [                                                   ]   0.00% (800.00 MHz)
...
```

Add `-i` to skip the utilization, which then needs no sampling interval and returns at once.

In addition to `cpu`, `c`, `cpus`, `core`, `cores`, `load`, `processor`, and `processors` are also acceptable.

#### Show Memory Stats

```bash
mprober memory
```

```
Memory [|||||||||||||$$$$$$$$$$$$$$$$$$$$$$$$$$ ] 21.36 GiB / 62.07 GiB (34.41%)
Swap   [                                        ] 12.40 MiB /  7.63 GiB ( 0.16%)
```

In the plain output `|` is the used memory, `$` the page cache and `#` the buffers. With colors they are three shades instead.

In addition to `memory`, `m`, `mem`, `f`,`free`, `memories`, `swap`, `ram`, `dram`, `ddr`, `cache`, `buffer`, `buffers`, `buf`, and `buff` are also acceptable.

#### Show Network Stats

```bash
mprober network
```

```
        Upload Rate | Uploaded Data | Download Rate | Downloaded Data
lo            0 B/s         7.94 MB           0 B/s           7.94 MB
enp1s0    1.30 KB/s       193.34 MB       4.18 KB/s           1.80 GB
docker0       0 B/s         6.27 KB           0 B/s              84 B
```

In addition to `network`, `n`, `net`, `networks`,`bandwidth`, and `traffic` are also acceptable.

#### Show Volume Stats

```bash
mprober volume
```

```
          Reading Rate | Read Data | Writing Rate | Written Data
nvme0n1p2        0 B/s     7.25 MB          0 B/s        7.25 MB
          [||||||||||||||||||                     ] 975.23 MB / 2.01 GB (48.46%)
nvme0n1p4        0 B/s    12.39 GB          0 B/s       12.39 GB
          [|||||||||                              ] 473.00 GB / 1.99 TB (23.77%)
```

Add `--mounts` to also list the mount points of each volume, and `-i` to skip the I/O rates.

In addition to `volume`, `v`, `storage`, `volumes`, `d`, `disk`, `disks`, `blk`, `block`, `blocks`, `mount`, `mounts`, `ssd`, and `hdd` are also acceptable.

#### Show Pressure (PSI)

```bash
mprober pressure
```

```
                                                           avg10   avg60  avg300
CPU    some [                                         ]    0.00%   0.00%   0.00%
CPU    full [                                         ]    0.00%   0.00%   0.00%
Memory some [                                         ]    0.00%   0.00%   0.00%
Memory full [                                         ]    0.00%   0.00%   0.00%
IO     some [|||                                      ]    6.31%   2.04%   0.88%
IO     full [||                                       ]    4.10%   1.22%   0.51%
```

PSI is the share of time tasks spent stalled waiting for a resource. Unlike the load average it tells a system which is merely busy apart from one which is actually short of CPU, memory or I/O. `some` is the time at least one task was stalled, `full` the time every non-idle task was. The bar follows `avg10`, which is the most immediate of the three.

In addition to `pressure`, `psi`, `stall`, and `pressures` are also acceptable.

#### Show cgroup Limits

```bash
mprober cgroup
```

```
cgroup /sys/fs/cgroup/system.slice/mprober.service

CPU
  limit     2.00 CPUs
  usage     24 minutes, and 36 seconds
  user      16 minutes, and 18 seconds
  system    8 minutes, and 17 seconds
  throttled 41 of 1500 periods, for 3 seconds

Memory [|||||||||||||||||                         ] 1.68 GiB / 4.00 GiB (42.06%)
Swap   0 B, not limited
PIDs   [                                                  ] 1251 / 75971 (1.65%)
```

This shows the CPU quota, the memory limit and the PID limit that the container or the cloud instance actually caps this machine at, which is often lower than what `cpu` and `memory` report for the host. A limit which is not set reads `not limited` and gets no bar, and the throttled counters tell whether the CPU quota is really in the way.

In addition to `cgroup`, `g`, `container`, `limit`, `limits`, and `cgroups` are also acceptable.

#### Show Processes

```bash
mprober process --top 5
```

```
   PID  PPID   PR  NI %CPU       VSZ       RSS       ANON THD TTY  USER
348084     1   25   5  4.2  75.2 MiB   5.7 MiB 1012.0 KiB   2      magiclen
105294  4025   20   0  0.0   1.4 TiB 478.3 MiB  315.6 MiB  34      magiclen
 18313 16498   20   0  0.0   1.4 TiB 423.5 MiB  278.6 MiB  32      magiclen
 16569 16498   20   0  0.0   1.4 TiB 381.2 MiB  242.5 MiB  31      magiclen
 51467 16498   20   0  0.0   1.4 TiB 439.6 MiB  302.5 MiB  31      magiclen
```

The rows are ordered by CPU and then by memory usage. `--pid-filter`, `--user-filter`, `--group-filter`, `--program-filter` and `--tty-filter` narrow the list, the last two by a regex. `-t` adds the start time of each process, and `--truncate` shortens the names.

In addition to `process`, `p`, and `ps` are also acceptable.

#### Benchmark

To benchmark the performance of CPU, memory and volumes,

```bash
mprober benchmark
```

```
Intel(R) Core(TM) Ultra 9 285K 24C/24T
5300 5106 5300 5300 5300 5300 5300 5300 4602 4602 4602 4602 4601 4602 4601 4601 4601 4601 4601 4601 4601 4601 4601 4601

CPU (multi-thread) : 1928195943.51 iterations/s
CPU (single thread): 104721455.14 iterations/s
Memory             : 126.11 GiB/s
```

The second line is the frequency of each core in MHz while the CPU was loaded, which is where a machine that cannot hold its boost clock shows up.

In addition to `benchmark`, `b`, `bench`, and `performance` are also acceptable.

Adding the `--disable-xxx` or `--enable-xxx` flags can control what benchmarks you want to run. The volume benchmark writes to each volume, so `--disable-volume` is worth knowing about.

### Web (HTTP)

#### Launching the Server

```bash
mprober web
```

In addition to `web`, `w`, `server`, and `http` are also acceptable.

Once you start the server, you can open [`http://0.0.0.0:8000`](http://0.0.0.0:8000) via a web browser such as Firefox or Chrome.

The page is one dashboard which follows the machine live, with a panel per subsystem: system identity, load average and per-core CPU, memory and swap, PSI, cgroup limits, network interfaces and volumes. A panel whose data the kernel does not provide says so rather than showing an error. It follows the light or dark theme of the browser, and can be toggled either way.

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