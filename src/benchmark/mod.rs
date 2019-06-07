use std::time::Duration;

use crate::scanner_rust::ScannerError;
use crate::byte_unit::{Byte, ByteUnit};
use crate::rand::{self, Rng};

use crate::cpu_info::CPU;
use crate::benchmarking;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BenchmarkLog {
    None,
    Normal,
    Verbose,
}

impl BenchmarkLog {
    #[inline]
    pub fn has_stdout(&self) -> bool {
        match self {
            BenchmarkLog::None => false,
            BenchmarkLog::Normal => true,
            BenchmarkLog::Verbose => true,
        }
    }

    #[inline]
    pub fn has_stderr(&self) -> bool {
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

impl ToString for BenchmarkError {
    fn to_string(&self) -> String {
        match self {
            BenchmarkError::ScannerError(error) => error.to_string(),
            BenchmarkError::BenchmarkError(error) => {
                match error {
                    benchmarking::BenchmarkError::MeasurerNotMeasured => "The measurer is not measured.".to_string()
                }
            }
            BenchmarkError::NoNeedBenchmark => {
                "There is nothing to benchmark".to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub warming_up_duration: Duration,
    pub benchmark_duration: Duration,
    pub print_out: BenchmarkLog,
    pub cpu: bool,
    pub memory: bool,
}

pub fn run_benchmark(config: &BenchmarkConfig) -> Result<(), BenchmarkError> {
    if !config.cpu && !config.memory {
        return Err(BenchmarkError::NoNeedBenchmark);
    }

    let cpus = CPU::get_cpus()?;

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
            let cpus = CPU::get_cpus()?;

            for cpu in cpus {
                println!("{} {}C/{}T", cpu.model_name, cpu.cpu_cores, cpu.siblings);

                let mut cpu_mhz_iter = cpu.cpus_mhz.into_iter();

                print!("{:.0}", cpu_mhz_iter.next().unwrap());

                loop {
                    if let Some(cpu_mhz) = cpu_mhz_iter.next() {
                        print!(" {:.0}", cpu_mhz);
                    } else {
                        break;
                    }
                }

                println!("\n");
            }
        }
    }

    // CPU
    {
        if config.cpu {
            if cpus_num > 1 {
                if config.print_out.has_stderr() {
                    eprintln!("Benchmarking CPU (multi-thread)... Please wait for {:?}.", config.benchmark_duration);
                }

                let bench_result = benchmarking::multi_thread_bench_function_with_duration(cpus_num, config.benchmark_duration, |measurer| {
                    let mut result = 0.0;

                    let mut divisor = 1.0;

                    let mut sum = 0;

                    measurer.measure_for_loop(0..1000_000, |loop_seq, _| {
                        let sub_result = 4.0 / divisor;

                        if loop_seq % 2 == 0 {
                            result += sub_result;
                        } else {
                            result -= sub_result;
                        }

                        divisor += 2.0;
                        sum += loop_seq;
                    });
                })?;

                if config.print_out.has_stdout() {
                    println!("CPU (multi-thread):  {:.2} iterations/s", bench_result.speed());

                    if config.print_out.has_stderr() {
                        eprintln!();
                    }
                }
            }

            if config.print_out.has_stderr() {
                eprintln!("Benchmarking CPU (single-thread)... Please wait for {:?}.", config.benchmark_duration);
            }

            let bench_result = benchmarking::bench_function_with_duration(config.benchmark_duration, |measurer| {
                let mut result = 0.0;

                let mut divisor = 1.0;

                let mut sum = 0;

                measurer.measure_for_loop(0..1000_000, |loop_seq, _| {
                    let sub_result = 4.0 / divisor;

                    if loop_seq % 2 == 0 {
                        result += sub_result;
                    } else {
                        result -= sub_result;
                    }

                    divisor += 2.0;
                    sum += loop_seq;
                });
            })?;

            if config.print_out.has_stdout() {
                println!("CPU (single thread): {:.2} iterations/s", bench_result.speed());
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

                eprintln!("Benchmarking memory... Please wait for {:?}.", config.benchmark_duration);
            }

            const MEM_SIZE: usize = 16 * 1024 * 1024; // N times of 4096

            let bench_result = benchmarking::bench_function_with_duration(config.benchmark_duration, |measurer| {
                let mut random = [0u8; 4096];

                let mut rng = rand::thread_rng();

                for i in 0..4096 {
                    random[i] = rng.gen();
                }

                let mut buffer = Vec::with_capacity(MEM_SIZE);

                unsafe {
                    buffer.set_len(MEM_SIZE);
                }

                measurer.measure_for_loop(0..(MEM_SIZE / 4096), |_, i| { // copy
                    let i = i * 4096;

                    buffer[i..(i + 4096)].copy_from_slice(&random);
                });

                buffer
            })?;

            let copy = {
                Byte::from_unit((bench_result.times() as f64 * 4096.0) / (bench_result.total_elapsed().as_nanos() as f64 / 1000000000.0), ByteUnit::B).unwrap().get_appropriate_unit(true).to_string()
            };

            if config.print_out.has_stdout() {
                println!("Memory:              {}/s", copy);
            }
        }
    }

    Ok(())
}