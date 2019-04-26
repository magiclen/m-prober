use std::io;
use std::fs;

const HOSTNAME_PATH: &'static str = "/etc/hostname";

pub fn get_hostname() -> Result<String, io::Error> {
    fs::read_to_string(HOSTNAME_PATH)
}