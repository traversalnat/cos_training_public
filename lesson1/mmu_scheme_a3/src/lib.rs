#![no_std]
#![feature(asm_const)]

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
const GIGA_PGSIZE: usize = 0x40000000;

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

// round a down to GIGA_PGSIZE
#[inline]
fn pg_round_down(a: usize) -> usize {
    a & !(GIGA_PGSIZE - 1)
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
    len: usize,
    prot: usize,
    alloc_page: &mut F1,
    _phys_to_virt: &F2,
) where
    F1: FnMut() -> *mut PageTable,
    F2: Fn(PAddr) -> *mut PageTable,
{
    let mut a = pg_round_down(va);
    let last = pg_round_down(va + len - 1);
    let mut pa = pa;

    loop {
        let mut pagetable = &mut *table;
        let mut idx = level - 1;
        // 只映射到 1GB 这一级, level = 2
        while idx > 2 {
            let pte = &mut pagetable.0[px(idx, a)];
            if *pte & 0x01 == 1 {
                pagetable = unsafe { &mut *pte2_pa(*pte) };
            } else {
                pagetable = unsafe { &mut *alloc_page() };
                *pte = pa2_pte(pagetable.0.as_ptr() as usize) | 0x01;
            }
            idx -= 1;
        }

        pagetable.0[px(idx, a)] = pa2_pte(pa) | prot | 0x01;

        if a == last {
            break;
        }

        a += GIGA_PGSIZE;
        pa += GIGA_PGSIZE;
    }
}

macro_rules! boot_map_pages {
    ( $( ($table:expr, $va:expr, $pa:expr, $len:expr, $prot:expr) ),* ) => {
        {
            $(
                boot_map($table, MMU_LEVELS, $va, $pa, $len, $prot, &mut alloc_page, &phys_to_virt);
            )*
        }

    };

}

pub unsafe fn pre_mmu() {
    let table = &mut PAGETABLE;
    boot_map_pages![
        (table, 0x8000_0000, 0x8000_0000, GIGA_PGSIZE, 0xef),
        (table, 0xffff_ffc0_8000_0000, 0x8000_0000, GIGA_PGSIZE, 0xef),
        (table, 0xffff_ffff_c000_0000, 0x8000_0000, GIGA_PGSIZE, 0xef)
    ];
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
