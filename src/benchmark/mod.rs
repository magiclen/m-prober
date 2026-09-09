use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    time::{Duration, SystemTime},
};

use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::*;
use rand::RngExt;

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
                    |measurer| {
                        let mut result = 0.0;

                        let mut divisor = 1.0;

                        let mut sum = 0usize;

                        for i in 0..1_000_000 {
                            measurer.measure(|| {
                                let sub_result = 4.0 / divisor;

                                if i % 2 == 0 {
                                    result += sub_result;
                                } else {
                                    result -= sub_result;
                                }

                                divisor += 2.0;
                                sum += i;
                            });
                        }
                    },
                )?;

                let speed = bench_result.speed();

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

            let bench_result = benchmarking::bench_function_with_duration(
                config.benchmark_duration,
                |measurer| {
                    let mut result = 0.0;

                    let mut divisor = 1.0;

                    let mut sum = 0usize;

                    for i in 0..1_000_000 {
                        measurer.measure(|| {
                            let sub_result = 4.0 / divisor;

                            if i % 2 == 0 {
                                result += sub_result;
                            } else {
                                result -= sub_result;
                            }

                            divisor += 2.0;
                            sum += i;
                        });
                    }
                },
            )?;

            let speed = bench_result.speed();

            cpu_single_thread = Some(speed);

            if config.print_out.has_stdout() {
                println!("CPU (single thread): {speed:.2} iterations/s");
            }
        }
    }

    // Memory
    {
        if config.memory {
            if config.print_out.has_stderr() {
                if config.cpu {
                    eprintln!();
                }

                eprintln!(
                    "Benchmarking memory... Please wait for {:?}.",
                    config.benchmark_duration
                );
            }

            const BUFFER_SIZE: usize = 4096;
            const MEM_SIZE: usize = 4 * BUFFER_SIZE; // N times of BUFFER_SIZE

            let bench_result = benchmarking::bench_function_with_duration(
                config.benchmark_duration,
                |measurer| {
                    let mut random = [0u8; BUFFER_SIZE];

                    let mut rng = rand::rng();

                    for e in random.iter_mut().take(BUFFER_SIZE) {
                        *e = rng.random();
                    }

                    let mut mem = [0u8; MEM_SIZE];

                    for i in 0..(MEM_SIZE / BUFFER_SIZE) {
                        let i = i * BUFFER_SIZE;

                        measurer.measure(|| {
                            mem[i..(i + BUFFER_SIZE)].copy_from_slice(&random);
                        });
                    }

                    mem
                },
            )?;

            let speed = bench_result.speed() * BUFFER_SIZE as f64;

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

/// The size of one read or write of the volume benchmark.
const VOLUME_BUFFER_SIZE: usize = 4096;

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
    let available = volume.size.saturating_sub(volume.used);

    if available <= TEST_FILE_SIZE || available - TEST_FILE_SIZE <= RESERVED_SIZE {
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

        let Ok(mut file) = File::create(&path) else {
            continue;
        };

        if config.print_out.has_stderr() {
            eprintln!(
                "Benchmarking {} ... Please wait for {:?}.",
                volume.device,
                config.benchmark_duration * 2
            );
        }

        let result = measure_file(&mut file, &path, config.benchmark_duration);

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
    duration: Duration,
) -> Result<(f64, f64), &'static str> {
    let write_result = measure_write(file, duration).ok_or("written")?;

    // Reading stops at the end of the file, so it has to hold at least one buffer.
    let file_size = stream_len(file).map_err(|_| "read")?;

    if file_size < TEST_FILE_SIZE {
        file.write_all(&[0u8; VOLUME_BUFFER_SIZE]).map_err(|_| "written")?;
    }

    file.seek(SeekFrom::Start(0)).map_err(|_| "read")?;

    let read_result = measure_read(path, duration).ok_or("read")?;

    Ok((read_result, write_result))
}

/// Write to the file for the whole duration, rewinding it once it has grown to the test size. `None` means a write failed, which leaves nothing worth reporting.
fn measure_write(file: &mut File, duration: Duration) -> Option<f64> {
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

        let buffer = [(measurer.get_seq() % 256) as u8; VOLUME_BUFFER_SIZE];

        measurer.measure(|| {
            if file.write_all(&buffer).is_err() {
                healthy = false;
            } else {
                file.flush().unwrap();
            }
        });
    })
    .ok()?;

    healthy.then(|| result.speed() * VOLUME_BUFFER_SIZE as f64)
}

/// Read the file back for the whole duration, rewinding it once the test size has been read. `None` means a read failed.
fn measure_read(path: &Path, duration: Duration) -> Option<f64> {
    let mut file = File::open(path).ok()?;

    let mut healthy = true;
    let mut laps = 1u128;

    let result = benchmarking::bench_function_with_duration(duration, |measurer| {
        if !healthy {
            measurer.measure(|| {});

            return;
        }

        if measurer.get_seq() * VOLUME_BUFFER_SIZE as u128 >= u128::from(TEST_FILE_SIZE) * laps {
            if file.seek(SeekFrom::Start(0)).is_err() {
                healthy = false;

                return;
            }

            laps += 1;
        }

        let mut buffer = [0u8; VOLUME_BUFFER_SIZE];

        measurer.measure(|| {
            if file.read_exact(&mut buffer).is_err() {
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
