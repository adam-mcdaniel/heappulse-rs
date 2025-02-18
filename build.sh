#!/bin/bash
# cargo clean
cargo build
# cargo build --release

# rm libcompress.so libcompress.dylib

# cp ./target/release/librust_compressor.a ./libcompress.a
cp ./target/debug/librust_compressor.a ./libcompress.a