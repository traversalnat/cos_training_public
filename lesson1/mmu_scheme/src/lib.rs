#![no_std]
#![feature(asm_const)]

use riscv::register::satp;

pub const KERNEL_BASE: usize = 0xffff_ffff_c000_0000;

const PHYS_VIRT_OFFSET: usize = 0xffff_ffc0_0000_0000;

#[link_section = ".data.boot_page_table"]
static mut BOOT_PT: [u64; 512] = [0; 512];

#[link_section = ".data.boot_page_table"]
static mut BOOT_PT_PMD: [u64; 512] = [0; 512];

#[cfg(feature = "sv39")]
pub unsafe fn pre_mmu() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    BOOT_PT[2] = (0x80000 << 10) | 0xef;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    BOOT_PT[0x102] = (0x80000 << 10) | 0xef;

    // 0xffff_ffff_c000_0000..highest, VRWX_GAD, 1G block
    BOOT_PT[0x1ff] = (0x80000 << 10) | 0xef;
}

#[cfg(feature = "sv48")]
pub unsafe fn pre_mmu() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    BOOT_PT_PMD[2] = (0x80000 << 10) | 0xef;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    BOOT_PT_PMD[0x102] = (0x80000 << 10) | 0xef;

    // 0xffff_ffff_c000_0000..highest, VRWX_GAD, 1G block
    BOOT_PT_PMD[0x1ff] = (0x80000 << 10) | 0xef;

    BOOT_PT[0] = ((BOOT_PT_PMD.as_ptr() as u64) << 10) | 0xef;

    BOOT_PT[0x1ff] = ((BOOT_PT_PMD.as_ptr() as u64) << 10) | 0xef;
}

pub unsafe fn enable_mmu() {
    let page_table_root = BOOT_PT.as_ptr() as usize;
    if cfg!(feature = "sv39") {
        satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    } else if cfg!(feature = "sv48") {
        satp::set(satp::Mode::Sv48, 0, page_table_root >> 12);
    }
    riscv::asm::sfence_vma_all();
}

pub unsafe fn post_mmu() {
    core::arch::asm!("
        li      t0, {phys_virt_offset}  // fix up virtual high address
        add     sp, sp, t0
        add     ra, ra, t0
        ret     ",
        phys_virt_offset = const PHYS_VIRT_OFFSET,
    )
}
