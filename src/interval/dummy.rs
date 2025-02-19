use crate::track::Block;

use super::IntervalTest;

pub struct DummyIntervalTest;

impl IntervalTest for DummyIntervalTest {
    fn name(&self) -> &str {
        "Dummy Interval Test"
    }

    fn boxed(&self) -> Box<dyn IntervalTest> {
        Box::new(Self)
    }

    fn on_write(&mut self, block: &Block) {
        tracing::info!("Write to block: {:?}", block);
        tracing::info!("Physical address: {:?}", block.physical_address());
    }
}