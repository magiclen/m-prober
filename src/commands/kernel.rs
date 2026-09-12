use mprober_lib::kernel;

#[inline]
pub fn handle_kernel() -> anyhow::Result<()> {
    let kernel_version = kernel::get_kernel_version()?;

    println!("{kernel_version}");

    Ok(())
}
