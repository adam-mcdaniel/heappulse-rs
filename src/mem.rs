extern crate libc;
use core::ffi::c_void;
use libc::{size_t, SA_SIGINFO, siginfo_t, ucontext_t, SIGBUS, SIGSEGV, sigaction, sighandler_t};

use crate::{INTERVAL_CONFIG, globals::*, interval::IntervalTest, logger::init_logging, track::{Block, Permissions}};
// Store original malloc and free function pointers
static mut ORIGINAL_MALLOC: Option<extern "C" fn(size_t) -> *mut c_void> = None;
static mut ORIGINAL_FREE: Option<extern "C" fn(*mut c_void)> = None;
static mut ORIGINAL_MMAP: Option<extern "C" fn(*mut c_void, size_t, i32, i32, i32, i32) -> *mut c_void> = None;
static mut ORIGINAL_MUNMAP: Option<extern "C" fn(*mut c_void, size_t) -> i32> = None;

pub fn align_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + page_size - 1) & !(page_size - 1)
}

pub fn align_down_to_page_size(size: usize, page_size: usize) -> usize {
    size & !(page_size - 1)
}

/// Signal handler for SIGSEGV/SIGBUS
extern "C" fn sigsegv_handler(sig: i32, info: *mut siginfo_t, context: *mut c_void) {
    tracing::error!("⚠️ Caught signal: {} (Segfault or Bus Error)", sig);
    if is_in_hook() {
        tracing::error!("Already in hook, exiting signal handler");
        std::process::exit(1);
    } else {
        enter_hook();
        get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);
    }

    #[cfg(target_arch = "x86_64")]
    let si_addr = unsafe { (*info).si_addr() as *const u8 };
    #[cfg(target_arch = "aarch64")]
    let si_addr = unsafe { (*info).si_addr as *const u8 };

    if !context.is_null() {
        let ucontext = context as *mut ucontext_t;
        // Get whether the fault was a read or write
        #[cfg(target_arch = "x86_64")]
        let is_write = unsafe { ((*ucontext).uc_mcontext).gregs[libc::REG_ERR as usize] & 0x2 != 0}; // Instruction Pointer

        #[cfg(target_arch = "aarch64")]
        let is_write = unsafe {detect_faulting_operation((*(*ucontext).uc_mcontext).__ss.__pc as *const u8) == Some("WRITE")}; // Program Counter

        tracing::trace!("Is write?: {:?}", is_write);
        tracing::trace!("Faulting address: {:?}", si_addr);
        match get_tracked_allocation(si_addr as *const u8) {
            Some(allocation) => {
                get_interval_test_suite_mut().on_access(&allocation, is_write);
                tracing::trace!("Faulting address is part of allocation: {:?}", allocation);
                if !is_write {
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ);
                } else {
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
                }
            },
            None => {
                tracing::error!("Faulting address is not part of any tracked allocation");
                std::process::exit(1);
            }
        }
    }

    exit_hook();
}

// Detect whether the faulting instruction was a read or a write
unsafe fn detect_faulting_operation(ip: *const u8) -> Option<&'static str> {
    if ip.is_null() {
        tracing::error!("Instruction Pointer is null");
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
        tracing::error!("Instruction: {:#010x}", instr);

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
    if ORIGINAL_MALLOC.is_some() && ORIGINAL_FREE.is_some() {
        return;
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

    ORIGINAL_MALLOC = Some(core::mem::transmute::<_, extern "C" fn(size_t) -> *mut c_void>(malloc_sym));
    ORIGINAL_FREE = Some(core::mem::transmute::<_, extern "C" fn(*mut c_void)>(free_sym));
    ORIGINAL_MMAP = Some(core::mem::transmute::<_, extern "C" fn(*mut c_void, size_t, i32, i32, i32, i32) -> *mut c_void>(mmap_sym));
    ORIGINAL_MUNMAP = Some(core::mem::transmute::<_, extern "C" fn(*mut c_void, size_t) -> i32>(munmap_sym));
}

fn original_malloc(mut size: size_t) -> *mut c_void {
    unsafe {
        init_hooks();
        if let Some(original_malloc) = ORIGINAL_MALLOC {
            size = if crate::ALIGN_ALLOCATIONS_TO_PAGE_SIZE {
                align_up_to_page_size(size as usize, crate::page_size())
            } else {
                size
            };
            original_malloc(size)
        } else {
            panic!("Original malloc not initialized");
        }
    }
}

fn original_free(ptr: *mut c_void) {
    unsafe {
        init_hooks();
        if let Some(original_free) = ORIGINAL_FREE {
            original_free(ptr);
        } else {
            panic!("Original free not initialized");
        }
    }
}

fn original_mmap(addr: *mut c_void, length: size_t, prot: i32, flags: i32, fd: i32, offset: i32) -> *mut c_void {
    unsafe {
        init_hooks();
        if let Some(original_mmap) = ORIGINAL_MMAP {
            original_mmap(addr, length, prot, flags, fd, offset)
        } else {
            panic!("Original mmap not initialized");
        }
    }
}

fn original_munmap(addr: *mut c_void, mut length: size_t) -> i32 {
    unsafe {
        init_hooks();
        if let Some(original_munmap) = ORIGINAL_MUNMAP {
            length = if crate::ALIGN_ALLOCATIONS_TO_PAGE_SIZE {
                align_up_to_page_size(length as usize, crate::page_size())
            } else {
                length
            };
            original_munmap(addr, length)
        } else {
            panic!("Original munmap not initialized");
        }
    }
}


static mut IN_HOOK: bool = false;
fn enter_hook() {
    // Mark a static variable denoting that we are in the hook
    tracing::trace!("Entering hook");
    unsafe {
        IN_HOOK = true;
    }
}

fn is_in_hook() -> bool {
    unsafe {
        IN_HOOK
    }
}

fn exit_hook() {
    // Mark a static variable denoting that we are no longer in the hook
    tracing::trace!("Exiting hook");
    unsafe {
        IN_HOOK = false;
    }
}

#[no_mangle]
pub extern "C" fn malloc(size: size_t) -> *mut c_void {
    if is_in_hook() {
        return original_malloc(size);
    } else {
        enter_hook();
    }
    // let size = align_up_to_page_size(size as usize, crate::page_size());
    tracing::trace!("Allocating {size} bytes", size = size);
    let ptr = original_malloc(size);

    match track_allocation(ptr as *mut u8, size) {
        Ok(true) => {
            tracing::warn!("Block {ptr:?} with size {size} tracked, already had previous entry");
        },
        Ok(false) => {
            tracing::trace!("Block {ptr:?} with size {size} tracked");
        },
        Err(e) => {
            tracing::error!("Failed to track allocation {ptr:?} with size {size}: {e:?}");
        }
    }
    if let Some(alloc) = get_tracked_allocation(ptr as *const u8) {
        get_interval_test_suite_mut().on_alloc(&alloc);
    }
    tracing::trace!("Allocations: {:#?}", get_tracked_allocations());
    
    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    if let Some(allocation) = get_tracked_allocation(ptr as *const u8) {
        allocation.change_permissions(Permissions::NONE);
    }
    

    exit_hook();
    ptr
}

#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    if is_in_hook() {
        return original_free(ptr);
    } else {
        enter_hook();
    }

    // libc::printf(b"[HOOKED] free(%p)\n\0".as_ptr() as *const i8, ptr);
    match track_deallocation(ptr as *const u8) {
        Ok(dealloc) => {
            // tracing::info!("Deallocation {ptr:?} tracked, already had previous entry", ptr = ptr);
            get_interval_test_suite_mut().on_dealloc(&dealloc);
        },
        Err(e) => {
            tracing::error!("Failed to track deallocation {ptr:?}: {e:?}");
        }
    }

    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

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

    let ptr = original_mmap(addr, length, prot, flags, fd, offset);
    if ptr.is_null() {
        tracing::error!("Failed to map memory");
        exit_hook();
        return ptr;
    }

    let size = align_up_to_page_size(length as usize, crate::page_size());
    tracing::trace!("Mapping {size} bytes at {ptr:?}", size = size, ptr = ptr);

    match track_allocation(ptr as *mut u8, size) {
        Ok(true) => {
            tracing::warn!("Block {ptr:?} with size {size} tracked, already had previous entry");
        },
        Ok(false) => {
            tracing::trace!("Block {ptr:?} with size {size} tracked");
        },
        Err(e) => {
            tracing::error!("Failed to track allocation {ptr:?} with size {size}: {e:?}");
        }
    }
    if let Some(alloc) = get_tracked_allocation(ptr as *const u8) {
        get_interval_test_suite_mut().on_alloc(&alloc);
    }
    tracing::trace!("Allocations: {:#?}", get_tracked_allocations());
    
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
    tracing::trace!("Unmapping {size} bytes at {ptr:?}", size = size, ptr = addr);

    match track_deallocation(addr as *const u8) {
        Ok(dealloc) => {
            // tracing::info!("Deallocation {ptr:?} tracked, already had previous entry", ptr = ptr);
            get_interval_test_suite_mut().on_dealloc(&dealloc);
        },
        Err(e) => {
            tracing::error!("Failed to track deallocation {addr:?}: {e:?}");
        }
    }

    get_interval_test_suite_mut().schedule(&INTERVAL_CONFIG);

    let ret = original_munmap(addr, length);
    exit_hook();
    ret
}