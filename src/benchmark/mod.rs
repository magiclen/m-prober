use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, File, OpenOptions},
    hint,
    io::{self, Read, Seek, SeekFrom, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, SystemTime},
};

use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::*;
use rand::Rng;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BenchmarkLog {
    #[allow(unused)]
    None,
    Normal,
    Verbose,
}

impl BenchmarkLog {
    #[inline]
    pub fn has_stdout(self) -> bool {
        match self {
            BenchmarkLog::None => false,
            BenchmarkLog::Normal => true,
            BenchmarkLog::Verbose => true,
        }
    }

    #[inline]
    pub fn has_stderr(self) -> bool {
        match self {
            BenchmarkLog::None => false,
            BenchmarkLog::Normal => false,
            BenchmarkLog::Verbose => true,
        }
    }
}

#[derive(Debug)]
pub enum BenchmarkError {
    ProbeError(mprober_lib::Error),
    #[allow(clippy::enum_variant_names)]
    BenchmarkError(benchmarking::BenchmarkError),
    IOError(io::Error),
    NoNeedBenchmark,
}

impl From<mprober_lib::Error> for BenchmarkError {
    #[inline]
    fn from(error: mprober_lib::Error) -> BenchmarkError {
        BenchmarkError::ProbeError(error)
    }
}

impl From<benchmarking::BenchmarkError> for BenchmarkError {
    #[inline]
    fn from(error: benchmarking::BenchmarkError) -> BenchmarkError {
        BenchmarkError::BenchmarkError(error)
    }
}

impl From<io::Error> for BenchmarkError {
    #[inline]
    fn from(error: io::Error) -> BenchmarkError {
        BenchmarkError::IOError(error)
    }
}

impl Display for BenchmarkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            BenchmarkError::ProbeError(error) => Display::fmt(error, f),
            BenchmarkError::BenchmarkError(error) => match error {
                benchmarking::BenchmarkError::MeasurerNotMeasured => {
                    f.write_str("The measurer is not measured.")
                },
            },
            BenchmarkError::IOError(error) => Display::fmt(error, f),
            BenchmarkError::NoNeedBenchmark => f.write_str("There is nothing to benchmark."),
        }
    }
}

impl Error for BenchmarkError {}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub cpu_multi_thread:  Option<f64>,
    pub cpu_single_thread: Option<f64>,
    pub memory:            Option<f64>,
    pub volumes:           Option<HashMap<String, (f64, f64)>>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub warming_up_duration: Duration,
    pub benchmark_duration:  Duration,
    pub print_out:           BenchmarkLog,
    pub cpu:                 bool,
    pub memory:              bool,
    pub volume:              bool,
}

/// How many terms one measured round covers.
///
/// A round is timed by two clock reads that cost more than a single term does, so measuring one term at a time would report how fast `Instant::now` is rather than how fast this CPU is.
const CPU_TERMS_PER_ROUND: usize = 1024;

/// How many rounds one call of the benchmarked closure runs, which the runner repeats until the duration is up.
const CPU_ROUNDS: usize = 1024;

/// Sum Leibniz's series for pi, which is a dependent chain of one division and one addition per term.
///
/// The sum is handed back so that the measurer's `black_box` keeps it, since a result nothing reads is a computation the compiler may drop.
fn cpu_terms(measurer: &mut benchmarking::Measurer) {
    let mut result = 0f64;
    let mut divisor = 1f64;
    let mut positive = true;

    for _ in 0..CPU_ROUNDS {
        measurer.measure(|| {
            for _ in 0..CPU_TERMS_PER_ROUND {
                let term = 4f64 / divisor;

                if positive {
                    result += term;
                } else {
                    result -= term;
                }

                positive = !positive;
                divisor += 2f64;
            }

            result
        });
    }

    hint::black_box(result);
}

/// How much bigger than the last-level cache the memory buffer is, so that reading it through has to reach the memory rather than the cache in front of it.
const MEMORY_CACHE_MULTIPLE: u64 = 2;

/// What one buffer falls back to when sysfs does not say how big the last-level cache is.
const MEMORY_FALLBACK_BUFFER: u64 = 64 * 1024 * 1024;

/// The share of the free memory the buffer may take, so that probing a small instance does not put it under pressure.
const MEMORY_BUDGET_SHARE: u64 = 4;

/// A floor, so that a machine with almost nothing free still measures something.
const MEMORY_MIN_BUFFER: u64 = 1024 * 1024;

/// How big the memory buffer should be: past the last-level cache, but never more than a share of what is free.
fn memory_buffer_size(cache: Option<u64>, available: u64) -> usize {
    let wanted = cache
        .map(|cache| cache.saturating_mul(MEMORY_CACHE_MULTIPLE))
        .unwrap_or(MEMORY_FALLBACK_BUFFER);

    let budget = available / MEMORY_BUDGET_SHARE;

    wanted.min(budget).max(MEMORY_MIN_BUFFER) as usize
}

/// Read every byte of the buffer, as a reduction the compiler turns into the widest loads it has.
///
/// Copying instead would hand the work to the C library's `memcpy`, and musl's is about half the speed of glibc's on a buffer this size, so the figure would say more about which binary is running than about the machine it runs on.
fn read_memory(buffer: &[u8]) -> u64 {
    // The buffer is a whole number of these, and the few bytes a remainder could hold would not move the rate anyway.
    let (chunks, _) = buffer.as_chunks::<{ size_of::<u64>() }>();

    chunks.iter().fold(0u64, |sum, chunk| sum.wrapping_add(u64::from_ne_bytes(*chunk)))
}

/// The size of the last-level cache in bytes, from sysfs.
///
/// `None` when the kernel does not expose it, e.g. in a container without `/sys` or on a platform which leaves these files out.
fn last_level_cache_size() -> Option<u64> {
    let mut largest: Option<(u32, u64)> = None;

    for entry in fs::read_dir("/sys/devices/system/cpu/cpu0/cache").ok()?.flatten() {
        let path = entry.path();

        let (Ok(level), Ok(size)) =
            (fs::read_to_string(path.join("level")), fs::read_to_string(path.join("size")))
        else {
            continue;
        };

        let (Ok(level), Some(size)) = (level.trim().parse::<u32>(), parse_cache_size(size.trim()))
        else {
            continue;
        };

        // One level holds an instruction cache and a data cache, and the larger of the two is the one a copy has to get past.
        if largest.is_none_or(|best| (level, size) > best) {
            largest = Some((level, size));
        }
    }

    largest.map(|(_, size)| size)
}

/// Parse what sysfs writes in a `size` file, e.g. `36864K`.
fn parse_cache_size(text: &str) -> Option<u64> {
    let (number, scale) = match text.strip_suffix('K') {
        Some(number) => (number, 1024),
        None => match text.strip_suffix('M') {
            Some(number) => (number, 1024 * 1024),
            None => (text, 1),
        },
    };

    number.parse::<u64>().ok().map(|number| number * scale)
}

pub fn run_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, BenchmarkError> {
    if !config.cpu && !config.memory && !config.volume {
        return Err(BenchmarkError::NoNeedBenchmark);
    }

    let cpus = cpu::get_cpus()?;

    let cpus_num = cpus.iter().map(|cpu| cpu.siblings).sum();

    // Warm up
    {
        if config.print_out.has_stderr() {
            eprintln!("Warming up... Please wait for {:?}.\n", config.warming_up_duration);
        }

        if config.cpu {
            benchmarking::warm_up_multi_thread_with_duration(cpus_num, config.warming_up_duration);
        } else {
            benchmarking::warm_up_with_duration(config.warming_up_duration);
        }

        if config.print_out.has_stdout() {
            let cpus = cpu::get_cpus()?;

            for cpu in cpus {
                let model_name =
                    cpu.model_name.as_deref().unwrap_or(crate::commands::UNKNOWN_CPU_MODEL_NAME);

                println!("{model_name} {}C/{}T", cpu.cpu_cores, cpu.siblings);

                // An architecture whose `/proc/cpuinfo` reports no `cpu MHz`, and which has no cpufreq files either, leaves nothing to print here.
                let mut cpu_mhz_iter = cpu.cpus_mhz.into_iter();

                if let Some(cpu_mhz) = cpu_mhz_iter.next() {
                    print!("{cpu_mhz:.0}");

                    for cpu_mhz in cpu_mhz_iter {
                        print!(" {cpu_mhz:.0}");
                    }
                }

                println!("\n");
            }
        }
    }

    let mut cpu_multi_thread = None;
    let mut cpu_single_thread = None;
    let mut memory = None;
    let mut volumes = None;

    // CPU
    {
        if config.cpu {
            if cpus_num > 1 {
                if config.print_out.has_stderr() {
                    eprintln!(
                        "Benchmarking CPU (multi-thread)... Please wait for {:?}.",
                        config.benchmark_duration
                    );
                }

                let bench_result = benchmarking::multi_thread_bench_function_with_duration(
                    cpus_num,
                    config.benchmark_duration,
                    cpu_terms,
                )?;

                let speed = bench_result.speed() * CPU_TERMS_PER_ROUND as f64;

                cpu_multi_thread = Some(speed);

                if config.print_out.has_stdout() {
                    println!("CPU (multi-thread) : {speed:.2} iterations/s");

                    if config.print_out.has_stderr() {
                        eprintln!();
                    }
                }
            }

            if config.print_out.has_stderr() {
                eprintln!(
                    "Benchmarking CPU (single-thread)... Please wait for {:?}.",
                    config.benchmark_duration
                );
            }

            let bench_result =
                benchmarking::bench_function_with_duration(config.benchmark_duration, cpu_terms)?;

            let speed = bench_result.speed() * CPU_TERMS_PER_ROUND as f64;

            cpu_single_thread = Some(speed);

            if config.print_out.has_stdout() {
                println!("CPU (single thread): {speed:.2} iterations/s");
            }
        }
    }

    // Memory
    {
        if config.memory {
            let cache = last_level_cache_size();
            let buffer_size = memory_buffer_size(cache, memory::free()?.mem.available);

            if config.print_out.has_stderr() {
                if config.cpu {
                    eprintln!();
                }

                eprintln!(
                    "Benchmarking memory over a buffer of {}... Please wait for {:?}.",
                    Byte::from_u64(buffer_size as u64).get_appropriate_unit(UnitType::Binary),
                    config.benchmark_duration
                );

                // A buffer the cache still swallows measures the cache, which is worth saying rather than reporting a figure that only looks good.
                if cache.is_some_and(|cache| buffer_size as u64 <= cache) {
                    eprintln!(
                        "There is not enough free memory to get past the cache, so this is the \
                         speed of the cache."
                    );
                }
            }

            // Random bytes rather than zeroes, since a hypervisor which shares identical pages would otherwise hand the whole buffer one page of memory, and touching every page here keeps the first round from faulting them in.
            let mut buffer = vec![0u8; buffer_size];

            rand::rng().fill_bytes(&mut buffer);

            let bench_result = benchmarking::bench_function_with_duration(
                config.benchmark_duration,
                |measurer| {
                    // The buffer holds the same bytes every round, so without a barrier the compiler would read it once and keep the answer.
                    measurer.measure(|| read_memory(hint::black_box(&buffer)));
                },
            )?;

            let speed = bench_result.speed() * buffer_size as f64;

            memory = Some(speed);

            if config.print_out.has_stdout() {
                let memory_result = format!(
                    "{:.2}",
                    Byte::from_f64(speed).unwrap().get_appropriate_unit(UnitType::Binary)
                );

                println!("Memory             : {memory_result}/s");
            }
        }
    }

    // Volume
    {
        if config.volume {
            let all_volumes = volume::get_volumes()?;

            if !all_volumes.is_empty() && config.print_out.has_stderr() {
                if config.cpu || config.memory {
                    eprintln!();
                }

                eprintln!("Benchmarking volumes...");
            }

            let mut volumes_result: HashMap<String, (f64, f64)> = HashMap::new();

            for volume in all_volumes {
                if let Some((read_result, write_result)) = benchmark_volume(&volume, config) {
                    if config.print_out.has_stdout() {
                        let read_result_string = format_speed(read_result);
                        let write_result_string = format_speed(write_result);

                        println!(
                            "{:<19}: Read {read_result_string}/s, Write {write_result_string}/s",
                            volume.device
                        );
                    }

                    volumes_result.insert(volume.device, (read_result, write_result));
                }
            }

            volumes = Some(volumes_result);
        }
    }

    Ok(BenchmarkResult {
        cpu_multi_thread,
        cpu_single_thread,
        memory,
        volumes,
    })
}

/// The size of one read or write of the volume benchmark, which is also the alignment `O_DIRECT` asks for.
const VOLUME_BUFFER_SIZE: usize = 4096;

/// A buffer `O_DIRECT` can be handed, which the kernel only accepts aligned to the logical block size of the device.
#[repr(align(4096))]
struct AlignedBuffer([u8; VOLUME_BUFFER_SIZE]);

impl AlignedBuffer {
    #[inline]
    fn new() -> Self {
        AlignedBuffer([0u8; VOLUME_BUFFER_SIZE])
    }
}

/// Open the test file, asking the kernel to keep the page cache out of the way.
///
/// Without `O_DIRECT` a benchmark this short never leaves the cache, and what it reports is the speed of the RAM in front of the device rather than of the device. A file system which does not implement the flag, e.g. tmpfs or a network mount, refuses to open at all, so the buffered path is still there to fall back on.
fn open_test_file(path: &Path, options: &mut OpenOptions) -> Option<(File, bool)> {
    if let Ok(file) = options.custom_flags(libc::O_DIRECT).open(path) {
        return Some((file, true));
    }

    options.custom_flags(0).open(path).ok().map(|file| (file, false))
}

/// How much is written before the file is rewound, so that a long benchmark does not fill the volume. It is a whole number of buffers.
const TEST_FILE_SIZE: u64 = 1024 * 1024 * 1024;

/// A volume is only benchmarked when this much would still be free afterwards.
const RESERVED_SIZE: u64 = 1024 * 1024 * 1024;

#[inline]
fn format_speed(bytes_per_second: f64) -> String {
    format!(
        "{:.2}",
        Byte::from_f64_with_unit(bytes_per_second, Unit::B)
            .unwrap()
            .get_appropriate_unit(UnitType::Binary)
    )
}

/// Returns the length of this stream (in bytes).
///
/// `Seek.stream_len(&mut self)` is unstable, so it is re-implemented here
fn stream_len(file: &mut File) -> Result<u64, io::Error> {
    let old_pos = file.stream_position()?;
    let len = file.seek(SeekFrom::End(0))?;

    if old_pos != len {
        file.seek(SeekFrom::Start(old_pos))?;
    }

    Ok(len)
}

/// Measure the read and the write speed of one volume, in bytes per second.
///
/// Only the first mount point that can be written to is measured, since every other one leads to the
/// same device and would only measure it again.
fn benchmark_volume(volume: &volume::Volume, config: &BenchmarkConfig) -> Option<(f64, f64)> {
    if volume.available <= TEST_FILE_SIZE + RESERVED_SIZE {
        if config.print_out.has_stderr() {
            eprintln!("{} doesn't have enough space to benchmark!", volume.device);
        }

        return None;
    }

    for point in volume.points.iter() {
        let path = Path::new(point).join(format!(
            "mprober-{}.tmp",
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()
        ));

        let Some((mut file, direct)) =
            open_test_file(&path, OpenOptions::new().write(true).create(true).truncate(true))
        else {
            continue;
        };

        if config.print_out.has_stderr() {
            eprintln!(
                "Benchmarking {} ... Please wait for {:?}.",
                volume.device,
                config.benchmark_duration * 2
            );

            if !direct {
                eprintln!(
                    "{} does not support O_DIRECT, so its figures are those of the page cache.",
                    volume.device
                );
            }
        }

        let result = measure_file(&mut file, &path, direct, config.benchmark_duration);

        drop(file);

        try_delete(&path);

        return match result {
            Ok(speeds) => Some(speeds),
            Err(stage) => {
                if config.print_out.has_stderr() {
                    eprintln!("{} cannot be {stage} successfully!", volume.device);
                }

                None
            },
        };
    }

    if config.print_out.has_stderr() {
        eprintln!("{} cannot be written!", volume.device);
    }

    None
}

/// Measure one file, writing to it first and then reading it back. The error names the step that failed.
fn measure_file(
    file: &mut File,
    path: &Path,
    direct: bool,
    duration: Duration,
) -> Result<(f64, f64), &'static str> {
    let write_result = measure_write(file, duration).ok_or("written")?;

    let mut file_size = stream_len(file).map_err(|_| "read")?;

    // Reading stops at the end of the file, so it has to hold at least one buffer.
    if file_size < VOLUME_BUFFER_SIZE as u64 {
        let buffer = AlignedBuffer::new();

        file.seek(SeekFrom::End(0)).map_err(|_| "written")?;
        file.write_all(&buffer.0).map_err(|_| "written")?;

        file_size += VOLUME_BUFFER_SIZE as u64;
    }

    let read_result = measure_read(path, direct, file_size, duration).ok_or("read")?;

    Ok((read_result, write_result))
}

/// Write to the file for the whole duration, rewinding it once it has grown to the test size. `None` means a write failed, which leaves nothing worth reporting.
fn measure_write(file: &mut File, duration: Duration) -> Option<f64> {
    let mut buffer = AlignedBuffer::new();

    let mut healthy = true;
    let mut laps = 1u128;

    let result = benchmarking::bench_function_with_duration(duration, |measurer| {
        if !healthy {
            measurer.measure(|| {});

            return;
        }

        if measurer.get_seq() * VOLUME_BUFFER_SIZE as u128 > u128::from(TEST_FILE_SIZE) * laps {
            if file.seek(SeekFrom::Start(0)).is_err() {
                healthy = false;

                return;
            }

            laps += 1;
        }

        buffer.0.fill((measurer.get_seq() % 256) as u8);

        measurer.measure(|| {
            if file.write_all(&buffer.0).is_err() {
                healthy = false;
            }
        });
    })
    .ok()?;

    healthy.then(|| result.speed() * VOLUME_BUFFER_SIZE as f64)
}

/// Read the file back for the whole duration, rewinding it once it has been read to the end. `None` means a read failed.
///
/// A benchmark too short to fill a whole `TEST_FILE_SIZE` leaves a smaller file behind, so turning around at that size instead would read past the end and report the volume as unreadable.
fn measure_read(path: &Path, direct: bool, file_size: u64, duration: Duration) -> Option<f64> {
    let mut options = OpenOptions::new();

    options.read(true);

    let mut file = if direct {
        options.custom_flags(libc::O_DIRECT).open(path).ok()?
    } else {
        options.open(path).ok()?
    };

    // Whole buffers only, since the last part of the file is shorter than one and `read_exact` would fail on it.
    let lap_size = u128::from(file_size - file_size % VOLUME_BUFFER_SIZE as u64);

    let mut buffer = AlignedBuffer::new();

    let mut healthy = true;
    let mut laps = 1u128;

    let result = benchmarking::bench_function_with_duration(duration, |measurer| {
        if !healthy {
            measurer.measure(|| {});

            return;
        }

        if measurer.get_seq() * VOLUME_BUFFER_SIZE as u128 >= lap_size * laps {
            if file.seek(SeekFrom::Start(0)).is_err() {
                healthy = false;

                return;
            }

            laps += 1;
        }

        measurer.measure(|| {
            if file.read_exact(&mut buffer.0).is_err() {
                healthy = false;
            }
        });
    })
    .ok()?;

    healthy.then(|| result.speed() * VOLUME_BUFFER_SIZE as f64)
}

#[inline]
fn try_delete<P: AsRef<Path>>(path: P) {
    let _ = fs::remove_file(path.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_size() {
        assert_eq!(Some(36864 * 1024), parse_cache_size("36864K"));
        assert_eq!(Some(8 * 1024 * 1024), parse_cache_size("8M"));
        assert_eq!(Some(512), parse_cache_size("512"));
        assert_eq!(None, parse_cache_size("what"));
    }

    #[test]
    fn test_memory_buffer_size_gets_past_the_cache() {
        // Twice a 36 MiB last-level cache, on a machine with the memory to spare.
        assert_eq!(72 << 20, memory_buffer_size(Some(36 << 20), 32 << 30));
    }

    #[test]
    fn test_memory_buffer_size_of_a_small_instance() {
        // A quarter of what is free is all the buffer may have.
        assert_eq!(64 << 20, memory_buffer_size(Some(36 << 20), 256 << 20));
    }

    #[test]
    fn test_memory_buffer_size_without_a_cache_size() {
        assert_eq!(MEMORY_FALLBACK_BUFFER as usize, memory_buffer_size(None, 32 << 30));
    }
}
