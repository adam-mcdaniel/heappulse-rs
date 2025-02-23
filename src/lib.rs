pub mod hooks;
pub mod track;
pub mod globals;
pub mod logger;
pub mod interval;
pub mod config;
pub mod compress;
mod internal_alloc;

pub use config::*;

pub fn page_size() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_PAGESIZE) as usize
    }
}

pub fn align_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + page_size - 1) & !(page_size - 1)
}

pub fn align_down_to_page_size(size: usize, page_size: usize) -> usize {
    size & !(page_size - 1)
}
