// use heapless::Vec;

pub const MAX_COMPRESSED_SIZE: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    LZ4,
    Snappy,
}

impl CompressionAlgorithm {
    pub fn compress(&self, input: &impl AsRef<[u8]>) -> Option<Vec<u8>> {
        let input = input.as_ref();
        // let mut output = heapless::Vec::<u8, MAX_COMPRESSED_SIZE>::new();

        let mut output = Vec::<u8>::new();

        match self {
            CompressionAlgorithm::LZ4 => {
                // output.resize_default(lz4_flex::block::get_maximum_output_size(input.len())).ok()?;
                output.resize(lz4_flex::block::get_maximum_output_size(input.len()), Default::default());
                let compressed_size = lz4_flex::compress_into(input, &mut output).ok()?;
                output.truncate(compressed_size);
            }
            CompressionAlgorithm::Snappy => {
                let mut encoder = snap::raw::Encoder::new();
                // output.resize_default(MAX_COMPRESSED_SIZE).ok()?;
                output.resize(MAX_COMPRESSED_SIZE, Default::default());
                match encoder.compress(input, output.as_mut_slice()) {
                    Ok(compressed_size) => {
                        output.truncate(compressed_size);
                        tracing::debug!("Compressed Snappy data: {} bytes", compressed_size);
                    }
                    Err(e) => {
                        tracing::error!("Could not compress Snappy data: {e:?}");
                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    pub fn compress_in_place(&self, input: &mut impl AsMut<[u8]>) -> Option<usize> {
        let input = input.as_mut();
        let compressed = self.compress(&input)?;
        input[..compressed.len()].copy_from_slice(&compressed);
        input[compressed.len()..].fill(0);
        Some(compressed.len())
    }

    pub fn decompress(&self, input: &impl AsRef<[u8]>, compressed_size: usize) -> Option<Vec<u8>> {
        let input = input.as_ref();
        let mut output = Vec::<u8>::new();
        match self {
            CompressionAlgorithm::LZ4 => {
                // output.resize_default(lz4_flex::block::get_maximum_output_size(input.len())).ok()?;
                output.resize(lz4_flex::block::get_maximum_output_size(input.len()), Default::default());
                let input = &input[..compressed_size];
                match lz4_flex::decompress_into(input, &mut output) {
                    Ok(decompressed_size) => {
                        output.truncate(decompressed_size);
                        tracing::debug!("Decompressed LZ4 data: {} bytes", decompressed_size);
                    }
                    Err(e) => {
                        tracing::error!("Could not decompress LZ4 data: {e:?}");
                        return None;
                    }
                }
            }
            CompressionAlgorithm::Snappy => {
                let mut decoder = snap::raw::Decoder::new();
                let input = &input[..compressed_size];
                // output.resize_default(snap::raw::decompress_len(input).ok()?).ok()?;
                output.resize(snap::raw::decompress_len(input).ok()?, Default::default());
                match decoder.decompress(input, output.as_mut_slice()) {
                    Ok(decompressed_size) => {
                        output.truncate(decompressed_size);
                        tracing::debug!("Decompressed Snappy data: {} bytes", decompressed_size);
                    }
                    Err(e) => {
                        tracing::error!("Could not decompress Snappy data: {e:?}");
                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    pub fn decompress_in_place(&self, input: &mut impl AsMut<[u8]>, compressed_size: usize) -> Option<usize> {
        let input = input.as_mut();
        let decompressed = self.decompress(&input, compressed_size)?;
        input[..decompressed.len()].copy_from_slice(&decompressed);
        input[decompressed.len()..].fill(0);
        Some(decompressed.len())
    }
}