use std::io;
use std::fs;

const HOSTNAME_PATH: &'static str = "/proc/sys/kernel/hostname";

pub fn get_hostname() -> Result<String, io::Error> {
    fs::read_to_string(HOSTNAME_PATH)
}