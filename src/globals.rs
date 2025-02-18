use spin::RwLock;
use super::track::{Track, Block};

pub const MAX_TRACKED_ALLOCATIONS: usize = 1024;

pub static TRACK: RwLock<Track<MAX_TRACKED_ALLOCATIONS>> = RwLock::new(Track::new());

pub fn track_allocation(ptr: *mut u8, size: usize) -> Result<bool, Block> {
    TRACK.write().insert(Block::new(ptr, size))
}

pub fn get_tracked_allocation(ptr: *const u8) -> Option<Block> {
    TRACK.read().get(ptr)
}

pub fn track_deallocation(ptr: *const u8) -> Result<Block, ()> {
    TRACK.write().remove_ptr(ptr)
}

pub fn get_tracked_allocations<>() -> Track<MAX_TRACKED_ALLOCATIONS> {
    TRACK.read().clone()
}