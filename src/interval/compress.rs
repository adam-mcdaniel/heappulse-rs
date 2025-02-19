use crate::{MAX_TRACKED_ALLOCATIONS, compress::CompressionAlgorithm, globals::get_tracked_allocations, track::Block};
use heapless::FnvIndexMap as IndexMap;
use super::IntervalTest;
use tracing::*;

#[derive(Clone)]
pub struct CompressAlloc {
    algo: CompressionAlgorithm,
    compressed_sizes: IndexMap<*const u8, usize, MAX_TRACKED_ALLOCATIONS>,
}

impl CompressAlloc {
    pub fn new(algo: CompressionAlgorithm) -> Self {
        Self {
            algo,
            compressed_sizes: IndexMap::new(),
        }
    }

    pub fn is_compressed(&self, block: &Block) -> bool {
        self.compressed_sizes.contains_key(&block.ptr())
    }

    pub fn compress_all_allocations(&mut self) {
        let tracked = get_tracked_allocations();
        for mut block in tracked.into_iter() {
            if let Some(compressed_size) = block.compress(self.algo) {
                info!("    Compressed block: {:?} to {} bytes", block, compressed_size);
                self.compressed_sizes.insert(block.ptr(), compressed_size).unwrap();
                block.protect();
            }
        }
    }

    pub fn decompress_allocation(&mut self, mut block: Block) {
        let ptr = block.ptr();
        if let Some(&compressed_size) = self.compressed_sizes.get(&ptr) {
            if block.decompress(self.algo, compressed_size).is_some() {
                info!("    Successfully decompressed block: {:?}", block);
                self.compressed_sizes.remove(&ptr);
            } else {
                error!("    Could not decompress block: {:?}", block);
            }
        } else {
            error!("    Could not find compressed size for block: {:?}", block);
        }
    }
}

impl IntervalTest for CompressAlloc {
    fn name(&self) -> &str {
        "Compression Alloc Interval Test"
    }

    fn boxed(&self) -> Box<dyn IntervalTest> {
        Box::new(self.clone())
    }

    fn on_access(&mut self, block: &Block, _is_write: bool) {
        // Decompress the block
        if self.is_compressed(block) {
            info!("Got access to block: {:?}, decompressing", block);
            self.decompress_allocation(block.clone());
        }
    }

    fn on_interval(&mut self) {
        // Compress all allocations
        self.compress_all_allocations();
    }
}