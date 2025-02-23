use std::collections::HashSet;

use super::*;

pub struct IntervalSuite {
    last_interval: Instant,
    total_intervals_executed: u64,
    tests: Vec<Box<dyn Interval>>,
}

impl IntervalSuite {
    pub const fn new() -> Self {
        Self { tests: Vec::new(), last_interval: unsafe {core::mem::zeroed()}, total_intervals_executed: 0 }
    }

    pub fn from_tests(tests: &[Box<dyn Interval>]) -> Self {
        let mut suite = Self::new();
        for test in tests.iter() {
            suite.add_test(test.as_ref());
        }
        suite
    }

    pub fn add_test(&mut self, test: &dyn Interval) {
        // self.tests.push(test.boxed()).map_err(|_| ()).expect("Failed to add test");
        self.tests.push(test.boxed());
    }

    fn is_ready(&self, config: &IntervalConfig) -> bool {
        if self.total_intervals_executed == 0 {
            return true;
        }
        let elapsed = self.last_interval.elapsed().as_millis();
        elapsed >= config.interval_ms as u128
    }

    pub fn schedule(&mut self, config: &IntervalConfig) {
        if self.is_ready(config) {
            self.total_intervals_executed += 1;
            tracing::info!("Running tests for interval #{}", self.total_intervals_executed);
            let mut to_remove = Vec::<usize>::new();

            self.unprotect_allocations();
            
            self.last_interval = Instant::now();
            for (i, test) in self.tests.iter_mut().enumerate() {
                tracing::info!("Running test: {}", test.name());
                test.on_interval();

                if test.is_done() {
                    tracing::info!("Test {} is done, removing", test.name());
                    // Remove the test
                    // to_remove.push(i).expect("Failed to remove test");
                    to_remove.push(i);
                }
            }

            for i in to_remove.iter() {
                self.tests.remove(*i);
            }

            tracing::info!("Interval #{} complete", self.total_intervals_executed);
            self.protect_allocations();
        }
    }

    fn protect_allocations(&self) {
        tracing::info!("Protecting all allocations");
        let blocks = get_tracked_allocations();
        let mut pages = HashSet::new();

        for block in blocks.iter() {
            pages.extend(block.pages());
            // block.change_permissions(Permissions::NONE);
        }

        for page in pages {
            page.change_permissions(Permissions::NONE);
        }
    }

    fn unprotect_allocations(&self) {
        tracing::info!("Unprotecting all allocations");
        // let blocks = get_tracked_allocations();
        // for block in blocks.iter() {
        //     block.change_permissions(Permissions::READ | Permissions::WRITE);
        // }

        let blocks = get_tracked_allocations();
        let mut pages = HashSet::new();

        for block in blocks.iter() {
            pages.extend(block.pages());
            // block.change_permissions(Permissions::NONE);
        }

        for page in pages {
            page.change_permissions(Permissions::NONE);
        }
    }
}

impl Interval for IntervalSuite {
    fn name(&self) -> &str {
        "IntervalSuite"
    }

    fn boxed(&self) -> Box<dyn Interval> {
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

unsafe impl Send for IntervalSuite {}
unsafe impl Sync for IntervalSuite {}