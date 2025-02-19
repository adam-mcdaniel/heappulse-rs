use std::time::Instant;
use heapless::Vec;

use crate::{
    track::{Block, Permissions},
    globals::get_tracked_allocations
};

pub mod dummy;
pub use dummy::*;

pub mod dummy_compress;
pub use dummy_compress::*;

pub mod compress;
pub use compress::*;

pub trait IntervalTest {
    fn name(&self) -> &str;

    fn is_done(&self) -> bool {
        false
    }

    fn boxed(&self) -> Box<dyn IntervalTest>;

    fn on_alloc(&mut self, alloc: &Block) {
        tracing::info!("Found alloc: {:?}", alloc);
    }
    fn on_dealloc(&mut self, dealloc: &Block) {
        tracing::info!("Found dealloc: {:?}", dealloc);
    }

    fn on_access(&mut self, block: &Block, is_write: bool) {
        tracing::info!("Accessing block: {:?}", block);
    }
    fn on_write(&mut self, block: &Block) {
        tracing::info!("Writing to block: {:?}", block);
    }
    fn on_read(&mut self, block: &Block) {
        tracing::info!("Reading from block: {:?}", block);
    }

    fn on_interval(&mut self) {
        tracing::info!("Interval test: {}", self.name());
    }
}

pub const MAX_INTERVAL_TESTS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct IntervalTestConfig {
    pub interval_ms: u64,
}

pub struct IntervalTestSuite {
    last_interval: Instant,
    total_intervals_executed: u64,
    tests: Vec<Box<dyn IntervalTest>, MAX_INTERVAL_TESTS>,
}

impl IntervalTestSuite {
    pub const fn new() -> Self {
        Self { tests: Vec::new(), last_interval: unsafe {core::mem::zeroed()}, total_intervals_executed: 0 }
    }

    pub fn from_tests(tests: &[Box<dyn IntervalTest>]) -> Self {
        let mut suite = Self::new();
        for test in tests.iter() {
            suite.add_test(test.as_ref());
        }
        suite
    }

    pub fn add_test(&mut self, test: &dyn IntervalTest) {
        self.tests.push(test.boxed()).map_err(|_| ()).expect("Failed to add test");
    }

    fn is_ready(&self, config: &IntervalTestConfig) -> bool {
        if self.total_intervals_executed == 0 {
            return true;
        }
        let elapsed = self.last_interval.elapsed().as_millis();
        elapsed >= config.interval_ms as u128
    }

    pub fn schedule(&mut self, config: &IntervalTestConfig) {
        if self.is_ready(config) {
            self.total_intervals_executed += 1;
            tracing::info!("Running tests for interval #{}", self.total_intervals_executed);
            let mut to_remove = Vec::<usize, MAX_INTERVAL_TESTS>::new();

            self.unprotect_allocations();
            
            self.last_interval = Instant::now();
            for (i, test) in self.tests.iter_mut().enumerate() {
                tracing::info!("Running test: {}", test.name());
                test.on_interval();

                if test.is_done() {
                    tracing::info!("Test {} is done, removing", test.name());
                    // Remove the test
                    to_remove.push(i).expect("Failed to remove test");
                }
            }

            for i in to_remove.iter() {
                self.tests.remove(*i);
            }

            tracing::info!("Interval #{} complete", self.total_intervals_executed);
        }
        self.protect_allocations();
    }

    fn protect_allocations(&self) {
        tracing::trace!("Protecting all allocations");
        let blocks = get_tracked_allocations();
        for block in blocks.iter() {
            block.change_permissions(Permissions::NONE);
        }
    }

    fn unprotect_allocations(&self) {
        tracing::trace!("Unprotecting all allocations");
        let blocks = get_tracked_allocations();
        for block in blocks.iter() {
            block.change_permissions(Permissions::READ | Permissions::WRITE);
        }
    }
}

impl IntervalTest for IntervalTestSuite {
    fn name(&self) -> &str {
        "IntervalTestSuite"
    }

    fn boxed(&self) -> Box<dyn IntervalTest> {
        Box::new(Self::new())
    }

    fn on_alloc(&mut self, alloc: &Block) {
        alloc.unprotect();
        for test in self.tests.iter_mut() {
            test.on_alloc(alloc);
        }
        alloc.protect();
    }

    fn on_dealloc(&mut self, dealloc: &Block) {
        dealloc.unprotect();
        for test in self.tests.iter_mut() {
            test.on_dealloc(dealloc);
        }
        dealloc.protect();
    }

    fn on_access(&mut self, block: &Block, is_write: bool) {
        block.unprotect();
        for test in self.tests.iter_mut() {
            test.on_access(block, is_write);
            if is_write {
                test.on_write(block);
            } else {
                test.on_read(block);
            }
        }
        block.protect();
    }

    fn on_write(&mut self, block: &Block) {
        block.unprotect();
        for test in self.tests.iter_mut() {
            test.on_write(block);
        }
        block.protect();
    }

    fn on_read(&mut self, block: &Block) {
        block.unprotect();
        for test in self.tests.iter_mut() {
            test.on_read(block);
        }
        block.protect();
    }
}

unsafe impl Send for IntervalTestSuite {}
unsafe impl Sync for IntervalTestSuite {}