/**
 * These mirror what `mprober-lib` serializes, so a `std::time::Duration` arrives as its seconds and
 * nanoseconds rather than as a single number.
 */
export interface Duration {
    secs: number;
    nanos: number;
}

interface Uptime {
    total_uptime: Duration;
    all_cpu_idle_time: Duration;
}

interface LoadAverage {
    one: number;
    five: number;
    fifteen: number;
}

interface Cpu {
    physical_id: number;
    /** `/proc/cpuinfo` has no `model name` field on some architectures. */
    model_name: string | null;
    cpus_mhz: number[];
    siblings: number;
    cpu_cores: number;
}

interface Mem {
    total: number;
    used: number;
    free: number;
    shared: number;
    buffers: number;
    cache: number;
    available: number;
}

interface Swap {
    total: number;
    used: number;
    free: number;
    cache: number;
}

interface Free {
    mem: Mem;
    swap: Swap;
}

interface NetworkStat {
    receive_bytes: number;
    receive_packets: number;
    receive_errors: number;
    receive_dropped: number;
    transmit_bytes: number;
    transmit_packets: number;
    transmit_errors: number;
    transmit_dropped: number;
}

interface NetworkSpeed {
    receive: number;
    transmit: number;
    receive_packets: number;
    transmit_packets: number;
}

interface NetworkWithSpeed {
    interface: string;
    stat: NetworkStat;
    speed: NetworkSpeed;
}

interface VolumeStat {
    reads_completed: number;
    read_bytes: number;
    read_time: Duration;
    writes_completed: number;
    write_bytes: number;
    write_time: Duration;
    io_in_progress: number;
    io_time: Duration;
    weighted_io_time: Duration;
    discards_completed: number;
    discard_bytes: number;
    discard_time: Duration;
    flushes_completed: number;
    flush_time: Duration;
}

interface VolumeSpeed {
    read: number;
    write: number;
    read_iops: number;
    write_iops: number;
    utilization: number;
    average_queue_length: number;
}

interface VolumeWithSpeed {
    device: string;
    stat: VolumeStat;
    size: number;
    used: number;
    available: number;
    free: number;
    inodes: number;
    inodes_free: number;
    fs_type: string;
    points: string[];
    speed: VolumeSpeed;
}

interface PressureStat {
    avg10: number;
    avg60: number;
    avg300: number;
    total: Duration;
}

export interface Pressure {
    some: PressureStat;
    /** The CPU has no `full` line on kernels older than 5.13. */
    full: PressureStat | null;
}

interface SystemPressure {
    cpu: Pressure;
    memory: Pressure;
    io: Pressure;
}

export interface CgroupCpu {
    quota: Duration | null;
    period: Duration | null;
    usage: Duration;
    user: Duration;
    system: Duration;
    nr_periods: number;
    nr_throttled: number;
    throttled: Duration;
}

interface CgroupMemory {
    current: number;
    peak: number | null;
    max: number | null;
    high: number | null;
    swap_current: number | null;
    swap_max: number | null;
}

interface CgroupPids {
    current: number;
    max: number | null;
}

interface CgroupSummary {
    path: string;
    cpu: CgroupCpu | null;
    memory: CgroupMemory | null;
    pids: CgroupPids | null;
}

export interface CpuThreadSnapshot {
    id: number;
    physical_id: number | null;
    usage: number | null;
    frequency_mhz: number | null;
}

export interface Snapshot {
    hostname: string;
    kernel: string;
    uptime: Uptime;
    rtc_time: string;
    load_average: LoadAverage;
    cpus: Cpu[];
    /** The first entry is the average over every CPU, the rest are the individual ones. */
    cpus_stat: number[];
    cpu_threads: CpuThreadSnapshot[];
    memory: Free;
    network: NetworkWithSpeed[];
    volumes: VolumeWithSpeed[];
    pressure: SystemPressure | null;
    cgroup: CgroupSummary | null;
}

export interface Config {
    version: string;
    detect_interval: number;
}

export interface AuthStatus {
    required: boolean;
    authenticated: boolean;
}
