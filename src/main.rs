mod benchmark;
mod cli;
mod commands;
mod cpu_sample;
mod terminal;
mod web;

use std::io;

use cli::*;
use commands::*;

fn main() -> anyhow::Result<()> {
    let result = run(get_args());

    // A signal puts the terminal back by itself, but an error ends a monitoring loop here instead.
    terminal::restore_terminal();

    match result {
        // A reader which stopped early, e.g. `head`, is not a failure of this program.
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe) =>
        {
            Ok(())
        },
        result => result,
    }
}

fn run(args: CLIArgs) -> anyhow::Result<()> {
    match &args.command {
        CLICommands::Hostname => handle_hostname()?,
        CLICommands::Kernel => handle_kernel()?,
        CLICommands::Uptime {
            ..
        } => handle_uptime(args)?,
        CLICommands::Time {
            ..
        } => handle_time(args)?,
        CLICommands::Cpu {
            ..
        } => handle_cpu(args)?,
        CLICommands::Memory {
            ..
        } => handle_memory(args)?,
        CLICommands::Network {
            ..
        } => handle_network(args)?,
        CLICommands::Volume {
            ..
        } => handle_volume(args)?,
        CLICommands::Pressure {
            ..
        } => handle_pressure(args)?,
        CLICommands::Cgroup {
            ..
        } => handle_cgroup(args)?,
        CLICommands::Process {
            ..
        } => handle_process(args)?,
        CLICommands::Web {
            ..
        } => handle_web(args)?,
        CLICommands::Benchmark {
            ..
        } => handle_benchmark(args)?,
    }

    Ok(())
}
