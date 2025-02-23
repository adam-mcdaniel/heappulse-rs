extern crate libc;
use core::ffi::c_void;
use libc::{size_t, SA_SIGINFO, siginfo_t, ucontext_t, SIGBUS, SIGSEGV, sigaction, sighandler_t};
use tracing::*;
use spin::{Mutex, RwLock};

use crate::{align_up_to_page_size, globals::*, interval::Interval, logger::init_logging, page_size, track::{Block, Permissions}, INTERVAL_CONFIG, UNPROTECT_READ_WRITE_ON_FAULT};

type MallocFn = extern "C" fn(size_t) -> *mut c_void;
type FreeFn = extern "C" fn(*mut c_void);
type MmapFn = extern "C" fn(*mut c_void, size_t, i32, i32, i32, i32) -> *mut c_void;
type MunmapFn = extern "C" fn(*mut c_void, size_t) -> i32;

// Store original malloc and free function pointers
static ORIGINAL_MALLOC: RwLock<Option<MallocFn>> = RwLock::new(None);
static ORIGINAL_FREE: RwLock<Option<FreeFn>> = RwLock::new(None);
static ORIGINAL_MMAP: RwLock<Option<MmapFn>> = RwLock::new(None);
static ORIGINAL_MUNMAP: RwLock<Option<MunmapFn>> = RwLock::new(None);

pub const SEGV_MAPERR: i32 = 1;
pub const SEGV_ACCERR: i32 = 2;

/// Signal handler for SIGSEGV/SIGBUS
extern "C" fn sigsegv_handler(sig: i32, info: *mut siginfo_t, context: *mut c_void) {
    static LAST_FAULT: RwLock<Option<usize>> = RwLock::new(None);

    #[cfg(target_arch = "x86_64")]
    let si_addr = unsafe { (*info).si_addr() as *const u8 };
    #[cfg(target_arch = "aarch64")]
    let si_addr = unsafe { (*info).si_addr as *const u8 };

    // error!("⚠️ Caught signal: {} (Segfault or Bus Error)", sig);
    trace!("Caught fault on protected memory (Signal {})", sig);
    if is_in_hook() {
        error!("Already in hook, exiting signal handler");
        trace!("Scheduled interval test suite");
        Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
        return;
        // std::process::exit(1);
    } else {
        enter_hook();
        // get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);
    }

    if !context.is_null() {
        let ucontext = context as *mut ucontext_t;
        // Get whether the fault was a read or write
        #[cfg(target_arch = "x86_64")]
        let is_write = unsafe { ((*ucontext).uc_mcontext).gregs[libc::REG_ERR as usize] & 0x2 != 0 };
        // let is_write = unsafe { detect_faulting_operation(((*ucontext).uc_mcontext).gregs[libc::REG_RIP as usize] as *const u8) == Some("WRITE")
        //     || ((*ucontext).uc_mcontext).gregs[libc::REG_ERR as usize] * 0x2 != 0}; // Instruction Pointer

        #[cfg(target_arch = "aarch64")]
        let is_write = unsafe {detect_faulting_operation((*(*ucontext).uc_mcontext).__ss.__pc as *const u8) == Some("WRITE")}; // Program Counter

        trace!("Is write?: {:?}", is_write);
        trace!("Faulting address: {:?}", si_addr);
        match get_tracked_allocation(si_addr as *const u8) {
            Some(allocation) => {
                /*
                get_interval_test_suite_mut().on_access(&allocation, is_write);
                trace!("Faulting address is part of allocation: {:?}", allocation);

                let mut last_fault = LAST_FAULT.read();
                let is_repeated_fault = si_addr == last_fault.unwrap_or(0) as *const u8;
                if is_repeated_fault {
                    warn!("Invalid access to already faulted address: {:?}", si_addr);
                    warn!("Unprotecting allocation for reads/writes: {:?}", allocation);
                }

                if UNPROTECT_READ_WRITE_ON_FAULT || is_write || is_repeated_fault {
                    trace!("Unprotecting allocation for reads/writes: {:?}", allocation);
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
                } else {
                    trace!("Unprotecting allocation for reads: {:?}", allocation);
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ);
                }

                let mut last_fault = LAST_FAULT.write();
                *last_fault = Some(si_addr as usize);
                 */

                get_interval_test_suite_mut().on_access(&allocation, is_write);

                if UNPROTECT_READ_WRITE_ON_FAULT || is_write {
                    trace!("Unprotecting allocation for reads/writes: {:?}", allocation);
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
                } else {
                    trace!("Unprotecting allocation for reads: {:?}", allocation);
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
                }
            },
            None => {
                error!("Faulting address is not part of any tracked allocation");
                std::process::exit(1);
            }
        }
    } else {
        error!("Context is null");
        Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
        return;
    }

    exit_hook();
}

// Detect whether the faulting instruction was a read or a write
unsafe fn detect_faulting_operation(ip: *const u8) -> Option<&'static str> {
    if ip.is_null() {
        error!("Instruction Pointer is null");
        return None;
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        match *ip {
            0x8B => Some("READ"),  // MOV opcode (reading from memory)
            0x89 => Some("WRITE"), // MOV opcode (writing to memory)
            _ => None,
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        let instr = *(ip as *const u32); // Read 32-bit instruction
        trace!("Instruction: {:#010x}", instr);

        // Check for LOAD instructions (LDR, LDRB, LDRH, LDRSW)
        if (instr & 0xFFC00000) == 0xB9400000 ||  // LDR (32-bit/64-bit)
           (instr & 0xFFC00000) == 0x39400000 ||  // LDRB (Load Byte)
           (instr & 0xFFC00000) == 0x79400000 {   // LDRH (Load Halfword)
            return Some("READ");
        }

        // Check for STORE instructions (STR, STRB, STRH)
        if (instr & 0xFFC00000) == 0xB9000000 ||  // STR (32-bit/64-bit)
           (instr & 0xFFC00000) == 0x39000000 ||  // STRB (Store Byte)
           (instr & 0xFFC00000) == 0x79000000 {   // STRH (Store Halfword)
            return Some("WRITE");
        }


        None
    }
}

/// Setup signal handler with SA_SIGINFO (for context capture)
unsafe fn setup_signal_handler() {
    let mut sa: sigaction = std::mem::zeroed();
    sa.sa_flags = SA_SIGINFO;
    sa.sa_sigaction = sigsegv_handler as sighandler_t;

    sigaction(SIGSEGV, &sa, core::ptr::null_mut());
    sigaction(SIGBUS, &sa, core::ptr::null_mut());
}

/// Initialize function pointers at load time
unsafe fn init_hooks() {
    init_logging();
    setup_signal_handler();
    // Get the original malloc, free, mmap, and munmap function pointers
    {
        let cached_malloc = ORIGINAL_MALLOC.read();
        let cached_free = ORIGINAL_FREE.read();
        let cached_mmap = ORIGINAL_MMAP.read();
        let cached_munmap = ORIGINAL_MUNMAP.read();
        if cached_mmap.is_some() && cached_munmap.is_some() {
            return;
        }
    }

    let malloc_sym = libc::dlsym(libc::RTLD_NEXT, b"malloc\0".as_ptr() as *const i8);
    let free_sym = libc::dlsym(libc::RTLD_NEXT, b"free\0".as_ptr() as *const i8);
    let mmap_sym = libc::dlsym(libc::RTLD_NEXT, b"mmap\0".as_ptr() as *const i8);
    let munmap_sym = libc::dlsym(libc::RTLD_NEXT, b"munmap\0".as_ptr() as *const i8);
    if malloc_sym.is_null() {
        panic!("Failed to load malloc or free");
    }
    if free_sym.is_null() {
        panic!("Failed to load free");
    }
    if mmap_sym.is_null() {
        panic!("Failed to load mmap");
    }
    if munmap_sym.is_null() {
        panic!("Failed to load munmap");
    }
    
    let mut cached_malloc = ORIGINAL_MALLOC.write();
    let mut cached_free = ORIGINAL_FREE.write();
    let mut cached_munmap = ORIGINAL_MUNMAP.write();
    let mut cached_mmap = ORIGINAL_MMAP.write();
    *cached_malloc = Some(core::mem::transmute::<_, MallocFn>(malloc_sym));
    *cached_free = Some(core::mem::transmute::<_, FreeFn>(free_sym));
    *cached_mmap = Some(core::mem::transmute::<_, MmapFn>(mmap_sym));
    *cached_munmap = Some(core::mem::transmute::<_, extern "C" fn(*mut c_void, size_t) -> i32>(munmap_sym));
}

pub fn original_malloc(mut size: size_t) -> *mut c_void {
    unsafe {
        init_hooks();
        // if let Some(original_mmap) = *ORIGINAL_MMAP.read() {
        //     size = if crate::ALIGN_ALLOCATIONS_TO_PAGE_SIZE {
        //         align_up_to_page_size(size as usize, crate::page_size())
        //     } else {
        //         size
        //     };
        //     original_mmap(core::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0)
        // } else {
        //     panic!("Original mmap not initialized");
        // }
        if let Some(original_malloc) = *ORIGINAL_MALLOC.read() {
            original_malloc(size)
        } else {
            panic!("Original malloc not initialized");
        }
    }
}

pub fn original_free(ptr: *mut c_void) {
    unsafe {
        init_hooks();
        if let Some(original_free) = *ORIGINAL_FREE.read() {
            original_free(ptr);
        } else {
            panic!("Original free not initialized");
        }
        // if let Some(original_munmap) = *ORIGINAL_MUNMAP.read() {
        //     original_munmap(ptr, 0);
        // } else {
        //     panic!("Original munmap not initialized");
        // }
    }
}


pub fn original_malloc_with_mmap(mut size: size_t) -> *mut c_void {
    original_mmap(core::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1, 0)
}

pub fn original_free_with_munmap(ptr: *mut c_void) {
    original_munmap(ptr, 0);
}


fn original_mmap(addr: *mut c_void, length: size_t, prot: i32, flags: i32, fd: i32, offset: i32) -> *mut c_void {
    info!("Original mmap called with addr: {:p}, length: {}, prot: {}, flags: {}, fd: {}, offset: {}", addr, length, prot, flags, fd, offset);
    unsafe {
        init_hooks();
        if let Some(original_mmap) = *ORIGINAL_MMAP.read() {
            original_mmap(addr, length, prot, flags, fd, offset)
        } else {
            panic!("Original mmap not initialized");
        }
    }
}

fn original_munmap(addr: *mut c_void, mut length: size_t) -> i32 {
    info!("Original munmap called with addr: {:p}, length: {}", addr, length);
    unsafe {
        init_hooks();
        if let Some(original_munmap) = *ORIGINAL_MUNMAP.read() {
            length = if crate::ALIGN_ALLOCATIONS_TO_PAGE_SIZE {
                align_up_to_page_size(length as usize, crate::page_size())
            } else {
                length
            };
            original_munmap(addr, 0)
        } else {
            panic!("Original munmap not initialized");
        }
    }
}



// static mut IN_HOOK: bool = false;
static IN_HOOK: Mutex<bool> = Mutex::new(false);
fn enter_hook() {
    // Mark a static variable denoting that we are in the hook
    // trace!("Entering hook with allocations: {:#?}", get_tracked_allocations());
    // unsafe {
    //     IN_HOOK = true;
    // }

    // Try to acquire the lock, if it's already locked, then we're already in the hook
    let mut lock = IN_HOOK.lock();
    if *lock {
        warn!("Already in hook, exiting");
        return;
    }
    *lock = true;
}

fn is_in_hook() -> bool {
    // unsafe {
    //     IN_HOOK
    // }
    *IN_HOOK.lock()
}

fn exit_hook() {
    // Mark a static variable denoting that we are no longer in the hook
    trace!("Exiting hook");
    // unsafe {
    //     IN_HOOK = false;
    // }
    *IN_HOOK.lock() = false;
}

#[no_mangle]
pub extern "C" fn malloc(size: size_t) -> *mut c_void {
    trace!("malloc({})", size);
    if is_in_hook() {
        warn!("Already in hook, exiting malloc");
        return original_malloc(size);
    } else {
        enter_hook();
    }
    // let size = align_up_to_page_size(size as usize, crate::page_size());
    info!("Allocating {size} bytes", size = size);
    let ptr = original_malloc(align_up_to_page_size(size, page_size()));

    match track_allocation(ptr as *mut u8, size) {
        Ok(true) => {
            tracing::warn!("Block {ptr:?} with size {size} tracked, already had previous entry");
        },
        Ok(false) => {
            info!("Block {ptr:?} with size {size} tracked");
        },
        Err(e) => {
            error!("Failed to track allocation {ptr:?} with size {size}: {e:?}");
        }
    }
    if let Some(alloc) = get_tracked_allocation(ptr as *const u8) {
        info!("Triggering on_alloc for {alloc:?}");
        // get_interval_test_suite_mut().on_alloc(&alloc);
        // info!("Triggered on_alloc for {alloc:?}");
    }
    // trace!("Allocations: {:#?}", get_tracked_allocations());
    
    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    // if let Some(allocation) = get_tracked_allocation(ptr as *const u8) {
    //     allocation.change_permissions(Permissions::NONE);
    // }

    exit_hook();
    ptr
}

#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    trace!("free({:?})", ptr);
    if is_in_hook() {
        // return original_free_with_munmap(ptr);
        original_free(ptr);
        return;
    } else {
        enter_hook();
    }

    // libc::printf(b"[HOOKED] free(%p)\n\0".as_ptr() as *const i8, ptr);
    match track_deallocation(ptr as *const u8) {
        Ok(dealloc) => {
            // info!("Deallocation {ptr:?} tracked, already had previous entry", ptr = ptr);
            get_interval_test_suite_mut().on_dealloc(&dealloc);
        },
        Err(e) => {
            error!("Failed to track deallocation {ptr:?}: {e:?}");
        }
    }

    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    // original_free_with_munmap(ptr);
    original_free(ptr);
    exit_hook();
}

// Now override mmap and munmap to track memory mappings
#[no_mangle]
pub extern "C" fn mmap(addr: *mut c_void, length: size_t, prot: i32, flags: i32, fd: i32, offset: i32) -> *mut c_void {
    if is_in_hook() {
        return original_mmap(addr, length, prot, flags, fd, offset);
    } else {
        enter_hook();
    }

    let size = align_up_to_page_size(length as usize, crate::page_size());
    let ptr = original_mmap(addr, size, prot, flags, fd, offset);
    if ptr.is_null() {
        error!("Failed to map memory");
        exit_hook();
        return ptr;
    }

    trace!("Mapping {size} bytes at {ptr:?}", size = size, ptr = ptr);

    match track_allocation(ptr as *mut u8, length as usize) {
        Ok(true) => {
            tracing::warn!("Block {ptr:?} with size {length} tracked, already had previous entry");
        },
        Ok(false) => {
            trace!("Block {ptr:?} with size {length} tracked");
        },
        Err(e) => {
            error!("Failed to track allocation {ptr:?} with size {length}: {e:?}");
        }
    }

    if let Some(alloc) = get_tracked_allocation(ptr as *const u8) {
        get_interval_test_suite_mut().on_alloc(&alloc);
    }
    trace!("Allocations: {:#?}", get_tracked_allocations());
    
    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    if let Some(allocation) = get_tracked_allocation(ptr as *const u8) {
        allocation.change_permissions(Permissions::NONE);
    }

    exit_hook();
    ptr
}

#[no_mangle]
pub extern "C" fn munmap(addr: *mut c_void, length: size_t) -> i32 {
    if is_in_hook() {
        return original_munmap(addr, length);
    } else {
        enter_hook();
    }

    let size = align_up_to_page_size(length as usize, crate::page_size());
    trace!("Unmapping {size} bytes at {ptr:?}", size = size, ptr = addr);

    match track_deallocation(addr as *const u8) {
        Ok(dealloc) => {
            // info!("Deallocation {ptr:?} tracked, already had previous entry", ptr = ptr);
            get_interval_test_suite_mut().on_dealloc(&dealloc);
        },
        Err(e) => {
            error!("Failed to track deallocation {addr:?}: {e:?}");
        }
    }

    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    let ret = original_munmap(addr, length);
    exit_hook();
    ret
}