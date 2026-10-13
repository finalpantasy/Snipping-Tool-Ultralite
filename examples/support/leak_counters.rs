//! Diagnostic-only counters. Nothing here is linked into the application.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
};
pub static LIVE: AtomicUsize = AtomicUsize::new(0);
static BLOCKS: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
struct Allocator;
fn add(bytes: usize) {
    let value = LIVE.fetch_add(bytes, Ordering::SeqCst) + bytes;
    PEAK.fetch_max(value, Ordering::SeqCst);
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            add(layout.size());
            BLOCKS.fetch_add(1, Ordering::SeqCst);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            add(layout.size());
            BLOCKS.fetch_add(1, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        System.dealloc(p, layout);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        BLOCKS.fetch_sub(1, Ordering::SeqCst);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = System.realloc(p, layout, size);
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
            add(size);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[cfg(windows)]
mod native {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
            },
            Threading::{
                GetCurrentProcess, GetCurrentProcessId, GetGuiResources, GetProcessHandleCount,
                GR_GDIOBJECTS,
            },
        },
    };
    #[repr(C)]
    #[derive(Default)]
    struct HeapSummary {
        cb: u32,
        allocated: usize,
        committed: usize,
        reserved: usize,
        max_reserve: usize,
    }
    #[repr(C)]
    struct ThreadEntry {
        size: u32,
        usage: u32,
        id: u32,
        owner: u32,
        base_priority: i32,
        delta_priority: i32,
        flags: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetProcessHeaps(count: u32, heaps: *mut *mut c_void) -> u32;
        fn HeapSummary(heap: *mut c_void, flags: u32, summary: *mut HeapSummary) -> i32;
        fn HeapSetInformation(
            heap: *mut c_void,
            class: i32,
            info: *const c_void,
            bytes: usize,
        ) -> i32;
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
        fn Thread32First(snapshot: *mut c_void, entry: *mut ThreadEntry) -> i32;
        fn Thread32Next(snapshot: *mut c_void, entry: *mut ThreadEntry) -> i32;
    }
    pub fn metrics() -> serde_json::Value {
        let rust_live = super::LIVE.load(super::Ordering::SeqCst);
        let rust_blocks = super::BLOCKS.load(super::Ordering::SeqCst);
        let rust_peak = super::PEAK.load(super::Ordering::SeqCst);
        let process = unsafe { GetCurrentProcess() };
        let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
        memory.cb = std::mem::size_of_val(&memory) as u32;
        unsafe {
            GetProcessMemoryInfo(
                process,
                (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
                memory.cb,
            )
            .unwrap()
        };
        let mut handles = 0;
        unsafe { GetProcessHandleCount(process, &mut handles).unwrap() };
        let gdi = unsafe { GetGuiResources(process, GR_GDIOBJECTS) };
        let mut heaps = [std::ptr::null_mut(); 256];
        let count = unsafe { GetProcessHeaps(heaps.len() as u32, heaps.as_mut_ptr()) };
        assert!(count > 0 && count as usize <= heaps.len());
        let (mut allocated, mut committed, mut reserved, mut failures) =
            (0usize, 0usize, 0usize, 0usize);
        let mut summaries = Vec::new();
        for heap in &heaps[..count as usize] {
            let mut summary = HeapSummary {
                cb: std::mem::size_of::<HeapSummary>() as u32,
                ..Default::default()
            };
            if unsafe { HeapSummary(*heap, 0, &mut summary) } != 0 {
                allocated += summary.allocated;
                committed += summary.committed;
                reserved += summary.reserved;
                summaries.push((summary.allocated, summary.committed, summary.reserved));
            } else {
                failures += 1;
            }
        }
        let snapshot = unsafe { CreateToolhelp32Snapshot(4, 0) };
        assert!(!snapshot.is_null() && snapshot as isize != -1);
        let mut entry = ThreadEntry {
            size: std::mem::size_of::<ThreadEntry>() as u32,
            usage: 0,
            id: 0,
            owner: 0,
            base_priority: 0,
            delta_priority: 0,
            flags: 0,
        };
        let owner = unsafe { GetCurrentProcessId() };
        let mut threads = 0;
        let mut next = unsafe { Thread32First(snapshot, &mut entry) };
        while next != 0 {
            if entry.owner == owner {
                threads += 1;
            }
            next = unsafe { Thread32Next(snapshot, &mut entry) };
        }
        unsafe { CloseHandle(HANDLE(snapshot)).unwrap() };
        serde_json::json!({"rust_live_bytes":rust_live,"rust_live_blocks":rust_blocks,"rust_peak_bytes":rust_peak,"private_bytes":memory.PrivateUsage,"working_set_bytes":memory.WorkingSetSize,"handles":handles,"threads":threads,"gdi":gdi,"heap_count":count,"heap_allocated_bytes":allocated,"heap_committed_bytes":committed,"heap_reserved_bytes":reserved,"heap_summary_failures":failures,"heaps":summaries})
    }
    /// A causal experiment confined to this diagnostic process; never app code.
    pub fn trim() {
        #[repr(C)]
        struct Optimize {
            version: u32,
            flags: u32,
        }
        let value = Optimize {
            version: 1,
            flags: 0,
        };
        assert_ne!(
            unsafe {
                HeapSetInformation(
                    std::ptr::null_mut(),
                    3,
                    (&value as *const Optimize).cast(),
                    std::mem::size_of_val(&value),
                )
            },
            0,
            "{}",
            std::io::Error::last_os_error()
        );
    }
}
pub fn metrics() -> serde_json::Value {
    #[cfg(windows)]
    {
        native::metrics()
    }
    #[cfg(not(windows))]
    {
        serde_json::json!({"rust_live_bytes":LIVE.load(Ordering::SeqCst)})
    }
}
pub fn trim() {
    #[cfg(windows)]
    native::trim();
}
