use crate::{globals::get_tracked_allocations, track::Block};

use super::Interval;

pub struct DummyInterval;

impl Interval for DummyInterval {
    fn name(&self) -> &str {
        "Dummy Interval Test"
    }

    fn boxed(&self) -> Box<dyn Interval> {
        Box::new(Self)
    }

    fn on_write(&mut self, block: &Block) {
        tracing::info!("Write to block: {:?}", block);
        tracing::info!("Physical address: {:?}", block.physical_address());
        tracing::info!("Allocations: {:?}", get_tracked_allocations());
    }
    
    fn on_read(&mut self, block: &Block) {
        tracing::info!("Read from block: {:?}", block);
        tracing::info!("Physical address: {:?}", block.physical_address());
        tracing::info!("Allocations: {:?}", get_tracked_allocations());
    }
}