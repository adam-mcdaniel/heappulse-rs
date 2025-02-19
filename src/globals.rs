use spin::RwLock;
use crate::interval::{CompressAlloc, DummyCompressIntervalTest, DummyIntervalTest, IntervalTest};

use super::track::{Track, Block};
use super::interval::{IntervalTestSuite};
use super::compress::CompressionAlgorithm;
use super::MAX_TRACKED_ALLOCATIONS;

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

lazy_static::lazy_static! {
    static ref INTERVAL_TEST_SUITE: RwLock<IntervalTestSuite> = RwLock::new(IntervalTestSuite::from_tests(&[
        // DummyIntervalTest.boxed(),
        // DummyCompressIntervalTest(CompressionAlgorithm::LZ4).boxed()
        // DummyCompressIntervalTest(CompressionAlgorithm::Snappy).boxed()
        // CompressAlloc::new(CompressionAlgorithm::Snappy).boxed()
        CompressAlloc::new(CompressionAlgorithm::LZ4).boxed()
    ]));
}

pub fn get_interval_test_suite<'a>() -> spin::RwLockReadGuard<'a, IntervalTestSuite> {
    INTERVAL_TEST_SUITE.read()
}

pub fn get_interval_test_suite_mut<'a>() -> spin::RwLockWriteGuard<'a, IntervalTestSuite> {
    INTERVAL_TEST_SUITE.write()
}