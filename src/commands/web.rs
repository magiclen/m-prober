use crate::{CLIArgs, CLICommands, web};

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
        web::serve(monitor, address, listen_port, auth_key, only_api)?;
    }

    Ok(())
}
