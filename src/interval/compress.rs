use crate::{MAX_TRACKED_ALLOCATIONS, compress::CompressionAlgorithm, globals::get_tracked_allocations, track::Block};
// use heapless::FnvIndexMap as IndexMap;
use std::collections::HashMap as IndexMap;
use super::Interval;
use tracing::*;

#[derive(Clone)]
pub struct CompressAlloc {
    algo: CompressionAlgorithm,
    // compressed_sizes: IndexMap<*const u8, usize, MAX_TRACKED_ALLOCATIONS>,
    compressed_sizes: IndexMap<*const u8, usize>,
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
                let _ = self.compressed_sizes.insert(block.ptr(), compressed_size);
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

impl Interval for CompressAlloc {
    fn name(&self) -> &str {
        "Compression Alloc Interval Test"
    }

    fn boxed(&self) -> Box<dyn Interval> {
        Box::new(self.clone())
    }

    fn on_access(&mut self, block: &Block, is_write: bool) {
        // Decompress the block
        if self.is_compressed(block) {
            if is_write {
                info!("Got write access to block: {:?}, decompressing", block);
            } else {
                info!("Got read access to block: {:?}, decompressing", block);
            }
            self.decompress_allocation(block.clone());
        } else {
            info!("Block: {:?} is not compressed, skipping!", block);
        }
    }

    fn on_interval(&mut self) {
        // Compress all allocations
        self.compress_all_allocations();
    }
}