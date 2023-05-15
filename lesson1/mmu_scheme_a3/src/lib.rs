#![no_std]
#![feature(asm_const)]

use core::assert_eq;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use riscv::register::satp;

#[cfg(feature = "sv39")]
const MMU_LEVELS: usize = 3;
#[cfg(feature = "sv48")]
const MMU_LEVELS: usize = 4;
#[cfg(feature = "sv57")]
const MMU_LEVELS: usize = 5;

pub const KERNEL_BASE: usize = 0xffff_ffff_c000_0000;

const PHYS_VIRT_OFFSET: usize = 0xffff_ffc0_0000_0000;

// 1GB page size
const GIGA_PGSIZE: usize = 0x100000;

#[derive(Clone, Copy)]
struct PageTable([usize; 512]);

// physical addr
type PAddr = usize;

#[link_section = ".data.boot_page_table"]
static mut PAGETABLE: PageTable = PageTable([0; 512]);

#[link_section = ".data.boot_page_table"]
static mut BOOT_PAGES: [PageTable; 4] = [PageTable([0; 512]); 4];

#[inline]
fn px(level: usize, va: usize) -> usize {
    let shift: usize = 12 + 9 * level;
    (va >> shift) & 0x1FF
}

#[inline]
fn pte2_pa(pte: usize) -> *mut PageTable {
    ((pte >> 10) << 12) as *mut PageTable
}

#[inline]
fn pa2_pte(pa: usize) -> usize {
    (pa >> 12) << 10
}

fn alloc_page() -> *mut PageTable {
    static PAGE_NO: AtomicUsize = AtomicUsize::new(0);
    let index = PAGE_NO.fetch_add(1, Ordering::Relaxed);
    unsafe { &mut BOOT_PAGES[index] as *mut PageTable }
}

// Convert a physical address to a virtual address with linear mapping.
const fn phys_to_virt(paddr: PAddr) -> *mut PageTable {
    (paddr + PHYS_VIRT_OFFSET) as *mut PageTable
}

fn boot_map<F1, F2>(
    table: &mut PageTable,
    level: usize,
    va: usize,
    pa: usize,
    _len: usize,
    prot: usize,
    alloc_page: &mut F1,
    _phys_to_virt: &F2,
) where
    F1: FnMut() -> *mut PageTable,
    F2: Fn(PAddr) -> *mut PageTable,
{
    let mut pagetable = table;
    let mut idx = level - 1;
    // 只映射到 1GB 这一级, level = 2
    while idx > 2 {
        let pte = &mut pagetable.0[px(idx, va)];
        if *pte & 0x01 == 1 {
            pagetable = unsafe { &mut *pte2_pa(*pte) };
        } else {
            pagetable = unsafe { &mut *alloc_page() };
            *pte = pa2_pte(pagetable.0.as_ptr() as usize) | 0x01;
        }
        idx -= 1;
    }

    pagetable.0[px(idx, va)] = pa2_pte(pa) | prot | 0x01;
}

pub unsafe fn pre_mmu() {
    let table = &mut PAGETABLE;
    boot_map(
        table,
        MMU_LEVELS,
        0x8000_0000,
        0x8000_0000,
        GIGA_PGSIZE,
        0xef,
        &mut alloc_page,
        &mut phys_to_virt,
    );
    boot_map(
        table,
        MMU_LEVELS,
        0xffff_ffc0_8000_0000,
        0x8000_0000,
        GIGA_PGSIZE,
        0xef,
        &mut alloc_page,
        &mut phys_to_virt,
    );
    boot_map(
        table,
        MMU_LEVELS,
        0xffff_ffff_c000_0000,
        0x8000_0000,
        GIGA_PGSIZE,
        0xef,
        &mut alloc_page,
        &mut phys_to_virt,
    );
}

pub unsafe fn enable_mmu() {
    let page_table_root = PAGETABLE.0.as_ptr() as usize;
    if cfg!(feature = "sv39") {
        satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    } else if cfg!(feature = "sv48") {
        satp::set(satp::Mode::Sv48, 0, page_table_root >> 12);
    } else if cfg!(feature = "sv57") {
        satp::set(satp::Mode::Sv57, 0, page_table_root >> 12);
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
