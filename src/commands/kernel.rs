use mprober_lib::kernel;

#[inline]
pub fn handle_kernel() {
    let kernel_version = kernel::get_kernel_version().unwrap();

    println!("{kernel_version}");
}
