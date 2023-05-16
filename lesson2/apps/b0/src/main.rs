#![no_std]
#![no_main]

use drv0 as _;
use drv1 as _;

use drv_common::CallEntry;

#[no_mangle]
fn main() {
    libos::init();

    libos::println!("\n[ArceOS Tutorial]: B0\n");
    verify();
}

// 在libos/linker_riscv64.lds中定义，用于指示 .init_calls 段的开始和结束地址
extern "C" {
    fn __init_calls_start();
    fn __init_calls_end();
}

fn traverse_drivers() {
    // Parse range of init_calls by calling C function.
    let range_start = __init_calls_start as usize;
    let range_end = __init_calls_end as usize;
    display_initcalls_range(range_start, range_end);

    let size = core::mem::size_of::<&CallEntry>();
    for addr in (range_start..range_end).step_by(size) {
        let entry = unsafe { core::mem::transmute::<*mut u8, &CallEntry>(addr as *mut u8) };
        let drv = (entry.init_fn)();
        // For each driver, display name & compatible
        display_drv_info(drv.name, drv.compatible);
    }
}

fn display_initcalls_range(start: usize, end: usize) {
    libos::println!("init calls range: 0x{:X} ~ 0x{:X}\n", start, end);
}

fn display_drv_info(name: &str, compatible: &str) {
    libos::println!("Found driver '{}': compatible '{}'", name, compatible);
}

fn verify() {
    traverse_drivers();

    libos::println!("\nResult: Okay!");
}
