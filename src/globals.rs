use spin::RwLock;

use super::track::{Track, Block};
use super::interval::{*, Interval, IntervalSuite};
use super::compress::CompressionAlgorithm;
use super::MAX_TRACKED_ALLOCATIONS;

pub static TRACK: RwLock<Option<Track<MAX_TRACKED_ALLOCATIONS>>> = RwLock::new(None);

fn init_tracking() {
    if TRACK.read().is_some() {
        return;
    }
    *TRACK.write() = Some(Track::new());
}

pub fn track_allocation(ptr: *mut u8, size: usize) -> Result<bool, Block> {
    init_tracking();
    TRACK.write().as_mut().unwrap().insert(Block::new(ptr, size))
}

pub fn get_tracked_allocation(ptr: *const u8) -> Option<Block> {
    init_tracking();
    TRACK.read().as_ref().unwrap().get(ptr)
}

pub fn track_deallocation(ptr: *const u8) -> Result<Block, ()> {
    init_tracking();
    TRACK.write().as_mut().unwrap().remove_ptr(ptr)
}

pub fn get_tracked_allocations() -> Track<MAX_TRACKED_ALLOCATIONS> {
    init_tracking();
    TRACK.read().as_ref().unwrap().clone()
}

lazy_static::lazy_static! {
    static ref INTERVAL_TEST_SUITE: RwLock<IntervalSuite> = RwLock::new(IntervalSuite::from_tests(&[
        DummyInterval.boxed(),
        // DummyCompressInterval(CompressionAlgorithm::LZ4).boxed()
        // DummyCompressInterval(CompressionAlgorithm::Snappy).boxed()
        // CompressAlloc::new(CompressionAlgorithm::Snappy).boxed()
        // crate::interval::CompressAlloc::new(CompressionAlgorithm::LZ4).boxed()
    ]));
}

pub fn get_interval_test_suite<'a>() -> spin::RwLockReadGuard<'a, IntervalSuite> {
    init_tracking();
    INTERVAL_TEST_SUITE.read()
}

pub fn get_interval_test_suite_mut<'a>() -> spin::RwLockWriteGuard<'a, IntervalSuite> {
    init_tracking();
    INTERVAL_TEST_SUITE.write()
}