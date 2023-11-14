use crate::{rocket_mounts, CLIArgs, CLICommands};

#[inline]
pub fn handle_web(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Web { .. }));

    if let CLICommands::Web {
        monitor,
        address,
        listen_port,
        auth_key,
        only_api,
    } = args.command
    {
        let rocket = rocket_mounts::create(monitor, address, listen_port, auth_key, only_api);

        rocket::execute(rocket.launch())?;
    }

    Ok(())
}
