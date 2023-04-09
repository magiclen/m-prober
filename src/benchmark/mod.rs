extern crate benchmarking;

use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
    time::{Duration, SystemTime},
};

use byte_unit::{Byte, ByteUnit};
use mprober_lib::*;
use rand::{self, Rng};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BenchmarkLog {
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
    ScannerError(ScannerError),
    BenchmarkError(benchmarking::BenchmarkError),
    IOError(io::Error),
    NoNeedBenchmark,
}

impl From<ScannerError> for BenchmarkError {
    #[inline]
    fn from(error: ScannerError) -> BenchmarkError {
        BenchmarkError::ScannerError(error)
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
            BenchmarkError::ScannerError(error) => Display::fmt(error, f),
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
                println!("{} {}C/{}T", cpu.model_name, cpu.cpu_cores, cpu.siblings);

                let mut cpu_mhz_iter = cpu.cpus_mhz.into_iter();

                print!("{:.0}", cpu_mhz_iter.next().unwrap());

                for cpu_mhz in cpu_mhz_iter {
                    print!(" {:.0}", cpu_mhz);
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
                    println!("CPU (multi-thread) : {:.2} iterations/s", speed);

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
                println!("CPU (single thread): {:.2} iterations/s", speed);
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

                    let mut rng = rand::thread_rng();

                    for e in random.iter_mut().take(BUFFER_SIZE) {
                        *e = rng.gen();
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
                let memory_result = Byte::from_unit(speed, ByteUnit::B)
                    .unwrap()
                    .get_appropriate_unit(true)
                    .to_string();

                println!("Memory             : {}/s", memory_result);
            }
        }
    }

    // Volume
    {
        if config.volume {
            let mut volumes_result: HashMap<String, (f64, f64)> = HashMap::new();

            const BUFFER_SIZE: usize = 4096;
            const TEST_FILE_SIZE: u64 = 1024 * 1024 * 1024; // N times of BUFFER_SIZE

            {
                let volumes = volume::get_volumes()?;

                if !volumes.is_empty() {
                    if config.print_out.has_stderr() {
                        if config.cpu || config.memory {
                            eprintln!();
                        }

                        eprintln!("Benchmarking volumes...");
                    }

                    for volume in volumes {
                        let available = volume.size - volume.used;

                        if available > TEST_FILE_SIZE
                            && available - TEST_FILE_SIZE > 1024 * 1024 * 1024
                        {
                            // preserve 1 GiB space
                            let mut can_write = false;

                            for point in volume.points {
                                let path = Path::new(&point).join(format!(
                                    "mprober-{}.tmp",
                                    SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                ));

                                match File::create(&path) {
                                    Ok(file) => {
                                        can_write = true;

                                        if config.print_out.has_stderr() {
                                            eprintln!(
                                                "Benchmarking {} ... Please wait for {:?}.",
                                                volume.device,
                                                config.benchmark_duration * 2
                                            );
                                        }

                                        let file = Rc::new(RefCell::new(file));
                                        let file_2 = file.clone();

                                        let write = Rc::new(RefCell::new((true, 1)));
                                        let write_2 = write.clone();

                                        let bench_result_r =
                                            benchmarking::bench_function_with_duration(
                                                config.benchmark_duration,
                                                move |measurer| {
                                                    if write_2.borrow().0 {
                                                        let mut file = file_2.borrow_mut();

                                                        let write = if measurer.get_seq()
                                                            * BUFFER_SIZE as u128
                                                            > u128::from(TEST_FILE_SIZE)
                                                                * write_2.borrow().1
                                                        {
                                                            if file
                                                                .seek(SeekFrom::Start(0))
                                                                .is_err()
                                                            {
                                                                write_2.borrow_mut().0 = false;

                                                                false
                                                            } else {
                                                                write_2.borrow_mut().1 += 1;

                                                                true
                                                            }
                                                        } else {
                                                            true
                                                        };

                                                        if write {
                                                            let buffer = [(measurer.get_seq() % 256)
                                                                as u8;
                                                                BUFFER_SIZE];

                                                            measurer.measure(|| {
                                                                if file.write_all(&buffer).is_err()
                                                                {
                                                                    write_2.borrow_mut().0 = false;
                                                                } else {
                                                                    file.flush().unwrap();
                                                                }
                                                            });
                                                        }
                                                    } else {
                                                        measurer.measure(|| {});
                                                    }
                                                },
                                            );

                                        if let Ok(bench_result) = bench_result_r {
                                            if write.borrow().0 {
                                                let write_result =
                                                    bench_result.speed() * BUFFER_SIZE as f64;
                                                let mut file = file.borrow_mut();

                                                /// Returns the length of this stream (in bytes).
                                                ///
                                                /// `Seek.stream_len(&mut self)` is unstable, so it is re-implemented here
                                                fn stream_len(
                                                    file: &mut File,
                                                ) -> Result<u64, io::Error>
                                                {
                                                    let old_pos = file.stream_position()?;
                                                    let len = file.seek(SeekFrom::End(0))?;

                                                    if old_pos != len {
                                                        file.seek(SeekFrom::Start(old_pos))?;
                                                    }

                                                    Ok(len)
                                                }

                                                match stream_len(&mut file) {
                                                    Ok(file_size) => {
                                                        let read = if file_size < TEST_FILE_SIZE {
                                                            let buffer = [0u8; BUFFER_SIZE];

                                                            if file.write_all(&buffer).is_err() {
                                                                if config.print_out.has_stderr() {
                                                                    eprintln!(
                                                                        "{} cannot be written \
                                                                         successfully!",
                                                                        volume.device
                                                                    );
                                                                }

                                                                false
                                                            } else {
                                                                true
                                                            }
                                                        } else {
                                                            true
                                                        };

                                                        let read = if read {
                                                            match file.seek(SeekFrom::Start(0)) {
                                                                Ok(_) => true,
                                                                Err(_) => {
                                                                    if config.print_out.has_stderr()
                                                                    {
                                                                        eprintln!(
                                                                            "{} cannot be read \
                                                                             successfully!",
                                                                            volume.device
                                                                        );
                                                                    }

                                                                    false
                                                                },
                                                            }
                                                        } else {
                                                            false
                                                        };

                                                        drop(file);

                                                        if read {
                                                            let read =
                                                                Rc::new(RefCell::new((true, 1)));
                                                            let read_2 = Rc::clone(&read);

                                                            match File::open(&path) {
                                                                Ok(mut file) => {
                                                                    let bench_result_r = benchmarking::bench_function_with_duration(config.benchmark_duration, move |measurer| {
                                                                        if read_2.borrow().0 {
                                                                            let read = if measurer.get_seq() * BUFFER_SIZE as u128 >= u128::from(TEST_FILE_SIZE) * read_2.borrow().1 {
                                                                                if file.seek(SeekFrom::Start(0)).is_err() {
                                                                                    read_2.borrow_mut().0 = false;

                                                                                    false
                                                                                } else {
                                                                                    read_2.borrow_mut().1 += 1;

                                                                                    true
                                                                                }
                                                                            } else {
                                                                                true
                                                                            };

                                                                            if read {
                                                                                let mut buffer = [0u8; BUFFER_SIZE];

                                                                                measurer.measure(|| {
                                                                                    if file.read_exact(&mut buffer).is_err() {
                                                                                        read_2.borrow_mut().0 = false;
                                                                                    }
                                                                                });
                                                                            }
                                                                        } else {
                                                                            measurer.measure(|| {});
                                                                        }
                                                                    });

                                                                    if let Ok(bench_result) =
                                                                        bench_result_r
                                                                    {
                                                                        if read.borrow().0 {
                                                                            let read_result =
                                                                                bench_result
                                                                                    .speed()
                                                                                    * BUFFER_SIZE
                                                                                        as f64;

                                                                            let read_result_string = Byte::from_unit(read_result, ByteUnit::B).unwrap().get_appropriate_unit(true).to_string();
                                                                            let write_result_string = Byte::from_unit(write_result, ByteUnit::B).unwrap().get_appropriate_unit(true).to_string();

                                                                            if config
                                                                                .print_out
                                                                                .has_stdout()
                                                                            {
                                                                                let mut s = volume
                                                                                    .device
                                                                                    .clone();

                                                                                let s_len = s.len();

                                                                                for _ in s_len..19 {
                                                                                    s.push(' ');
                                                                                }

                                                                                println!("{}: Read {}/s, Write {}/s", s, read_result_string, write_result_string);

                                                                                let s = {
                                                                                    let mut v = s
                                                                                        .into_bytes(
                                                                                        );

                                                                                    unsafe {
                                                                                        v.set_len(
                                                                                            s_len,
                                                                                        );

                                                                                        String::from_utf8_unchecked(v)
                                                                                    }
                                                                                };

                                                                                volumes_result.insert(s, (read_result, write_result));
                                                                            }
                                                                        } else if config
                                                                            .print_out
                                                                            .has_stderr()
                                                                        {
                                                                            eprintln!(
                                                                                "{} cannot be \
                                                                                 read successfully!\
                                                                                 ",
                                                                                volume.device
                                                                            );
                                                                        }
                                                                    } else {
                                                                        unreachable!();
                                                                    }
                                                                },
                                                                Err(_) => {
                                                                    if config.print_out.has_stderr()
                                                                    {
                                                                        eprintln!(
                                                                            "{} cannot be read \
                                                                             successfully!",
                                                                            volume.device
                                                                        );
                                                                    }
                                                                },
                                                            }
                                                        }
                                                    },
                                                    Err(_) => {
                                                        if config.print_out.has_stderr() {
                                                            eprintln!(
                                                                "{} cannot be read successfully!",
                                                                volume.device
                                                            );
                                                        }
                                                    },
                                                }
                                            } else if config.print_out.has_stderr() {
                                                eprintln!(
                                                    "{} cannot be written successfully!",
                                                    volume.device
                                                );
                                            }
                                        } else {
                                            unreachable!();
                                        }

                                        try_delete(path);
                                    },
                                    Err(_) => {
                                        continue;
                                    },
                                }
                            }

                            if !can_write && config.print_out.has_stderr() {
                                eprintln!("{} cannot be written!", volume.device);
                            }
                        } else if config.print_out.has_stderr() {
                            eprintln!("{} doesn't have enough space to benchmark!", volume.device);
                        }
                    }
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

#[inline]
fn try_delete<P: AsRef<Path>>(path: P) {
    if fs::remove_file(path.as_ref()).is_err() {}
}
