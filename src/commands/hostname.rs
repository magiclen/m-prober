use std::io::{self, Write};

use mprober_lib::hostname;

#[inline]
pub fn handle_hostname() -> anyhow::Result<()> {
    let hostname = hostname::get_hostname()?;

    // Unlike `println!`, this does not panic when the reader has gone away.
    writeln!(io::stdout(), "{hostname}")?;

    Ok(())
}
