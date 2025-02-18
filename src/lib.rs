pub mod mem;
pub mod track;
pub mod globals;
pub mod logger;


pub const ALIGN_ALLOCATIONS_TO_PAGE_SIZE: bool = true;


pub fn page_size() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_PAGESIZE) as usize
    }
}