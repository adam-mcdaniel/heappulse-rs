use crate::{compress::CompressionAlgorithm, globals::get_tracked_allocations, track::Block};

use super::IntervalTest;

#[derive(Clone, Copy)]
pub struct DummyCompressIntervalTest(pub CompressionAlgorithm);

impl IntervalTest for DummyCompressIntervalTest {
    fn name(&self) -> &str {
        "Dummy Compress Interval Test"
    }

    fn boxed(&self) -> Box<dyn IntervalTest> {
        Box::new(*self)
    }

    fn on_interval(&mut self) {
        let tracked = get_tracked_allocations();
        for mut block in tracked.into_iter() {
            tracing::info!("Found block: {block:?}");
            if let Some(compressed_size) = block.compress(self.0) {
                tracing::info!("Successfully compressed block: {block:?} to {compressed_size} bytes");

                if block.decompress(self.0, compressed_size).is_some() {
                    tracing::info!("Successfully compressed and decompressed block: {block:?}");
                } else {
                    tracing::error!("Could not decompress block: {block:?}");
                }
            } else {
                tracing::error!("Could not compress block: {block:?}");
            }
        }
    }
}