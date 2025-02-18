extern crate libc;
use core::ffi::c_void;
use libc::{size_t, mmap, SA_SIGINFO, siginfo_t, ucontext_t, mprotect, munmap, MAP_ANON, MAP_PRIVATE, PROT_NONE, PROT_READ, PROT_WRITE, SIGBUS, SIGSEGV, sigaction, sighandler_t};

use crate::{globals::*, logger::init_logging, track::{Block, Permissions}};
// Store original malloc and free function pointers
static mut ORIGINAL_MALLOC: Option<extern "C" fn(size_t) -> *mut c_void> = None;
static mut ORIGINAL_FREE: Option<extern "C" fn(*mut c_void)> = None;

pub fn align_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + page_size - 1) & !(page_size - 1)
}

pub fn align_down_to_page_size(size: usize, page_size: usize) -> usize {
    size & !(page_size - 1)
}

/// Signal handler for SIGSEGV/SIGBUS
extern "C" fn sigsegv_handler(sig: i32, info: *mut siginfo_t, context: *mut c_void) {
    tracing::error!("⚠️ Caught signal: {} (Segfault or Bus Error)", sig);

    #[cfg(target_arch = "x86_64")]
    let si_addr = unsafe { (*info).si_addr() as *const u8 };
    #[cfg(target_arch = "aarch64")]
    let si_addr = unsafe { (*info).si_addr as *const u8 };

    if !context.is_null() {
        let ucontext = context as *mut ucontext_t;
        // Get whether the fault was a read or write
        #[cfg(target_arch = "x86_64")]
        let ip = unsafe { ((*ucontext).uc_mcontext).gregs[libc::REG_RIP as usize] as *const u8 }; // Instruction Pointer

        #[cfg(target_arch = "aarch64")]
        let ip = unsafe {(*(*ucontext).uc_mcontext).__ss.__pc as *const u8}; // Program Counter

        println!("Segmentation fault at address: {:?}", ip);
        println!("Faulting instruction at: {:?}", ip);

        let operation = unsafe { detect_faulting_operation(ip) };
        match operation {
            Some("READ") => println!("Segfault caused by a READ operation."),
            Some("WRITE") => println!("Segfault caused by a WRITE operation."),
            _ => println!("Could not determine if it was a READ or WRITE."),
        }
        unsafe {
            tracing::error!("Faulting address: {:?}", si_addr);
            match get_tracked_allocation(si_addr as *const u8) {
                Some(allocation) => {
                    tracing::error!("Faulting address is part of allocation: {:?}", allocation);
                    Block::page_of(si_addr as *mut u8).change_permissions(Permissions::READ | Permissions::WRITE);
                },
                None => {
                    tracing::error!("Faulting address is not part of any tracked allocation");
                    std::process::exit(1);
                }
            }
        }
    }
}

// Detect whether the faulting instruction was a read or a write
unsafe fn detect_faulting_operation(ip: *const u8) -> Option<&'static str> {
    if ip.is_null() {
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
        if instr & 0x3B200C00 == 0x38200800 { 
            Some("READ")  // LDR (Load)
        } else if instr & 0x3B200C00 == 0x38200000 { 
            Some("WRITE") // STR (Store)
        } else {
            None
        }
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

    if malloc_sym.is_null() || free_sym.is_null() {
        panic!("Failed to load malloc or free");
    }

    ORIGINAL_MALLOC = Some(core::mem::transmute::<_, extern "C" fn(size_t) -> *mut c_void>(malloc_sym));
    ORIGINAL_FREE = Some(core::mem::transmute::<_, extern "C" fn(*mut c_void)>(free_sym));
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


static mut IN_HOOK: bool = false;
fn enter_hook() {
    // Mark a static variable denoting that we are in the hook
    tracing::info!("Entering hook");
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
    tracing::info!("Exiting hook");
    unsafe {
        IN_HOOK = false;
    }
}

/// macOS malloc override
#[no_mangle]
pub extern "C" fn malloc(size: size_t) -> *mut c_void {
    if is_in_hook() {
        return original_malloc(size);
    } else {
        enter_hook();
    }
    // let size = align_up_to_page_size(size as usize, crate::page_size());
    tracing::info!("Allocating {size} bytes", size = size);
    let ptr = original_malloc(size);

    // libc::printf(b"[HOOKED] malloc(%zu) -> %p\n\0".as_ptr() as *const i8, size, ptr);
    match track_allocation(ptr as *mut u8, size) {
        Ok(true) => {
            tracing::info!("Block {ptr:?} with size {size} tracked, already had previous entry");
        },
        Ok(false) => {
            // tracing::event!(
            //     tracing::Level::DEBUG,
            //     "Block {ptr:?} with size {size} tracked", ptr = ptr, size = size
            // );
            tracing::info!("Block {ptr:?} with size {size} tracked");
        },
        Err(e) => {
            // tracing::error!("Failed to track allocation {ptr:?} with size {size}: {e:?}");
        }
    }
    tracing::info!("Allocations: {:#?}", get_tracked_allocations());
    if let Some(allocation) = get_tracked_allocation(ptr as *const u8) {
        allocation.change_permissions(Permissions::NONE);
    }


    exit_hook();
    ptr
}

/// macOS free override
#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    if is_in_hook() {
        return original_free(ptr);
    } else {
        enter_hook();
    }
    original_free(ptr);

    // libc::printf(b"[HOOKED] free(%p)\n\0".as_ptr() as *const i8, ptr);
    match track_deallocation(ptr as *const u8) {
        Ok(alloc) => {
            // tracing::info!("Deallocation {ptr:?} tracked, already had previous entry", ptr = ptr);
        },
        Err(e) => {
            // tracing::error!("Failed to track deallocation {ptr:?}: {e:?}");
        }
    }
    exit_hook();
}