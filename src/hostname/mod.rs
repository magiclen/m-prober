use std::fs;
use std::io;

const HOSTNAME_PATH: &str = "/proc/sys/kernel/hostname";

#[inline]
pub fn get_hostname() -> Result<String, io::Error> {
    let mut s = fs::read_to_string(HOSTNAME_PATH)?;

    if s.ends_with('\n') {
        s.remove(s.len() - 1);
    }

    Ok(s)
}
