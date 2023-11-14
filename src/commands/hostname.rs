use mprober_lib::hostname;

#[inline]
pub fn handle_hostname() {
    let hostname = hostname::get_hostname().unwrap();

    println!("{hostname}");
}
