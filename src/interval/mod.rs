use std::time::Instant;
// use heapless::Vec;

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

pub mod suite;
pub use suite::*;

pub trait Interval {
    fn name(&self) -> &str;

    fn is_done(&self) -> bool {
        false
    }

    fn boxed(&self) -> Box<dyn Interval>;

    fn on_alloc(&mut self, alloc: &Block) {
        tracing::info!("Found alloc: {:?}", alloc);
    }
    fn on_dealloc(&mut self, dealloc: &Block) {
        tracing::info!("Found dealloc: {:?}", dealloc);
    }

    fn on_access(&mut self, block: &Block, _is_write: bool) {
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
pub struct IntervalConfig {
    pub interval_ms: u64,
}
