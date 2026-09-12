use mprober_lib::hostname;

#[inline]
pub fn handle_hostname() -> anyhow::Result<()> {
    let hostname = hostname::get_hostname()?;

    println!("{hostname}");

    Ok(())
}
