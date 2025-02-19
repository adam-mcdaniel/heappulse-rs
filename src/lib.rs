pub mod mem;
pub mod track;
pub mod globals;
pub mod logger;
pub mod interval;
pub mod config;
pub mod compress;

pub use config::*;

pub fn page_size() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_PAGESIZE) as usize
    }
}