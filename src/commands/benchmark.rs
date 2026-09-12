use crate::{CLIArgs, CLICommands, benchmark};

#[inline]
pub fn handle_benchmark(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Benchmark { .. }));

    if let CLICommands::Benchmark {
        warming_up_duration,
        benchmark_duration,
        verbose,
        disable_cpu,
        enable_cpu,
        disable_memory,
        enable_memory,
        disable_volume,
        enable_volume,
    } = args.command
    {
        let default = !(enable_cpu || enable_memory || enable_volume);

        let cpu = if disable_cpu { false } else { default || enable_cpu };

        let memory = if disable_memory { false } else { default || enable_memory };

        let volume = if disable_volume { false } else { default || enable_volume };

        let benchmark_config = benchmark::BenchmarkConfig {
            warming_up_duration,
            benchmark_duration,
            verbose,
            cpu,
            memory,
            volume,
        };

        benchmark::run_benchmark(&benchmark_config)?;
    }

    Ok(())
}
