// The game gives malloc only 32 MiB (nninitStartup, 0x71005BF120) and puts the rest of its memory in
// sead's heaps. The update borrows one block from the ExpHeap with the most free space through the
// heap's own tryAlloc and free, and returns it before the game continues
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::fmt::Write;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nookkit_sym::sym;
use nookkit_types::sead::{Heap, PtrArray, VftHeap};
use talc::{ErrOnOom, Span, Talc};

// Free space left in the lending heap, for the game's other threads
const HEAP_RESERVE: usize = 64 << 20;
const MAX_LOAN: usize = 1 << 30;
// Room for the decompressed shader archive (41 MiB in 3.0.3) next to a large model
const MIN_LOAN: usize = 128 << 20;
const LOAN_ALIGNMENT: i32 = 0x1000;
// Guards the heap walk against a corrupt list
const MAX_HEAPS: usize = 4096;

struct Block {
    talc: Talc<ErrOnOom>,
    base: usize,
    size: usize,
    live: usize,
}

pub struct Allocator {
    // The thread the block is lent to, 0 outside a loan
    owner: AtomicUsize,
    locked: AtomicBool,
    block: UnsafeCell<Option<Block>>,
}

// The block is only touched with `locked` held
unsafe impl Sync for Allocator {}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator {
    owner: AtomicUsize::new(0),
    locked: AtomicBool::new(false),
    block: UnsafeCell::new(None),
};

fn current_thread() -> usize {
    unsafe { skyline::nn::os::GetCurrentThread() as usize }
}

impl Allocator {
    fn with_block<T>(&self, f: impl FnOnce(&mut Option<Block>) -> T) -> T {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        let result = f(unsafe { &mut *self.block.get() });
        self.locked.store(false, Ordering::Release);
        result
    }

    fn lent_to_this_thread(&self) -> bool {
        let owner = self.owner.load(Ordering::Acquire);
        owner != 0 && owner == current_thread()
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.lent_to_this_thread() {
            return unsafe { System.alloc(layout) };
        }
        let borrowed = self.with_block(|block| {
            let block = block.as_mut()?;
            let pointer = unsafe { block.talc.malloc(layout) }.ok()?;
            block.live += 1;
            Some(pointer.as_ptr())
        });
        borrowed.unwrap_or_else(|| unsafe { System.alloc(layout) })
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // Frees from any thread, as memory from the block may be handed to another
        let borrowed = self.owner.load(Ordering::Acquire) != 0
            && self.with_block(|block| match block {
                Some(block)
                    if (block.base..block.base + block.size).contains(&(pointer as usize)) =>
                {
                    unsafe { block.talc.free(NonNull::new_unchecked(pointer), layout) };
                    block.live -= 1;
                    true
                }
                _ => false,
            });
        if !borrowed {
            unsafe { System.dealloc(pointer, layout) };
        }
    }
}

// Every sead heap, root heaps first, each followed by its descendants
unsafe fn heaps() -> Vec<*mut Heap> {
    let Ok(roots) = nookkit_sym::data::<PtrArray<Heap>>(&sym::sead::HeapMgr::sRootHeaps) else {
        return Vec::new();
    };
    let mut pending: Vec<_> = unsafe { (*roots).iter() }
        .filter(|h| !h.is_null())
        .collect();
    pending.reverse();
    let mut heaps = Vec::new();
    while let Some(heap) = pending.pop() {
        if heaps.len() == MAX_HEAPS {
            break;
        }
        heaps.push(heap);
        let children: Vec<_> = unsafe { (*heap).children() }.take(MAX_HEAPS).collect();
        pending.extend(children.into_iter().rev());
    }
    heaps
}

unsafe fn heap_name(heap: *mut Heap) -> &'static str {
    unsafe { (*heap).name.as_str() }.unwrap_or("?")
}

pub struct Loan {
    pub size: usize,
    // Each heap's name, size and largest free block, with the lending heap marked
    pub report: String,
}

// Only ExpHeaps lend: their free returns the block, unlike frame heaps
fn borrow(report: &mut String) -> Option<(*mut Heap, usize, usize)> {
    let exp_heap = nookkit_sym::vtable::<u8>(&sym::sead::ExpHeap::vtable).ok()? as usize;
    let mut lender: Option<(*mut Heap, usize)> = None;
    for heap in unsafe { heaps() } {
        let is_exp = unsafe { (*heap).base.vtable } as usize == exp_heap;
        let free = if is_exp {
            unsafe { (*heap).get_max_allocatable_size(LOAN_ALIGNMENT) }
        } else {
            0
        };
        let _ = writeln!(
            report,
            "  {} ({} MiB, {} MiB free{})",
            unsafe { heap_name(heap) },
            unsafe { (*heap).size } >> 20,
            free >> 20,
            if is_exp { "" } else { ", not an ExpHeap" }
        );
        if is_exp && lender.is_none_or(|(_, most)| free > most) {
            lender = Some((heap, free));
        }
    }
    let (heap, free) = lender?;
    let size = free.checked_sub(HEAP_RESERVE)?.min(MAX_LOAN) & !(LOAN_ALIGNMENT as usize - 1);
    if size < MIN_LOAN {
        return None;
    }
    // A negative alignment allocates from the heap's end, away from where the game allocates
    let base = unsafe { (*heap).try_alloc(size, -LOAN_ALIGNMENT) } as usize;
    let _ = writeln!(
        report,
        "Borrowing {} MiB from {}: {}",
        size >> 20,
        unsafe { heap_name(heap) },
        if base == 0 { "refused" } else { "done" }
    );
    (base != 0).then_some((heap, base, size))
}

// Runs `f` with this thread's allocations served from memory borrowed from the game's heaps. `f`
// must free everything it allocates, its result can't own heap memory
pub fn with_borrowed_memory<T: Copy>(f: impl FnOnce(&Loan) -> T) -> T {
    let mut report = String::from("sead heaps:\n");
    let Some((heap, base, size)) = borrow(&mut report) else {
        return f(&Loan { size: 0, report });
    };
    ALLOCATOR.with_block(|block| {
        let mut talc = Talc::new(ErrOnOom);
        // Claiming only fails for spans too small to hold the allocator's metadata
        let _ = unsafe { talc.claim(Span::new(base as *mut u8, (base + size) as *mut u8)) };
        *block = Some(Block {
            talc,
            base,
            size,
            live: 0,
        });
    });
    // The report is allocated before the loan, so it outlives it
    let loan = Loan { size, report };
    ALLOCATOR.owner.store(current_thread(), Ordering::Release);
    let result = f(&loan);
    ALLOCATOR.owner.store(0, Ordering::Release);
    let live = ALLOCATOR.with_block(|block| {
        let live = block.as_ref().map_or(0, |b| b.live);
        if live == 0 {
            *block = None;
        }
        live
    });
    if live == 0 {
        unsafe { (*heap).free(base as *mut _) };
    } else {
        // Freeing the block would corrupt what still lives in it, so the game loses this memory
        println!(
            "[NookKit] rstb: {live} allocations outlived the update, {} MiB stays borrowed",
            size >> 20
        );
    }
    result
}
