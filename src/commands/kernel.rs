use std::io::{self, Write};

use mprober_lib::kernel;

#[inline]
pub fn handle_kernel() -> anyhow::Result<()> {
    let kernel_version = kernel::get_kernel_version()?;

    // Unlike `println!`, this does not panic when the reader has gone away.
    writeln!(io::stdout(), "{kernel_version}")?;

    Ok(())
}
