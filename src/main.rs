#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_include_static_resources;

#[macro_use]
extern crate rocket_include_handlebars;

mod benchmark;
mod cli;
mod commands;
mod rocket_mounts;
mod terminal;

use cli::*;
use commands::*;
fn main() -> anyhow::Result<()> {
    let args = get_args();

    match &args.command {
        CLICommands::Hostname => handle_hostname(),
        CLICommands::Kernel => handle_kernel(),
        CLICommands::Uptime {
            ..
        } => handle_uptime(args),
        CLICommands::Time {
            ..
        } => handle_time(args),
        CLICommands::Cpu {
            ..
        } => handle_cpu(args),
        CLICommands::Memory {
            ..
        } => handle_memory(args),
        CLICommands::Network {
            ..
        } => handle_network(args),
        CLICommands::Volume {
            ..
        } => handle_volume(args),
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
