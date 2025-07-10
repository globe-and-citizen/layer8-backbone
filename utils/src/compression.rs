use std::io::prelude::*;
use std::str::FromStr;

use flate2::Compression;
use flate2::write::{GzEncoder, ZlibEncoder};

#[derive(Debug)]
pub enum CompressorVariant {
    Zlib,
    Gzip,
    // Deflate, // To be used when experimenting with with chunked data compression: <https://stackoverflow.com/a/10168441/10020745>
}

impl CompressorVariant {
    pub fn as_str(&self) -> &str {
        match self {
            CompressorVariant::Zlib => "zlib",
            CompressorVariant::Gzip => "gzip",
        }
    }
}

impl Default for CompressorVariant {
    fn default() -> Self {
        CompressorVariant::Zlib
    }
}

impl FromStr for CompressorVariant {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zlib" => Ok(CompressorVariant::Zlib),
            "gzip" => Ok(CompressorVariant::Gzip),
            _ => Ok(CompressorVariant::default()),
        }
    }
}

pub fn compress_data(variant: &CompressorVariant, data: &[u8]) -> Vec<u8> {
    match variant {
        CompressorVariant::Zlib => {
            // Compress using Zlib
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data)
                .expect("Failed to write data to Zlib encoder");
            encoder.finish().expect("Failed to finish Zlib encoding")
        }
        CompressorVariant::Gzip => {
            // Compress using Gzip
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data)
                .expect("Failed to write data to Gzip encoder");
            encoder.finish().expect("Failed to finish Gzip encoding")
        }
    }
}

pub fn decompress_data(variant: &CompressorVariant, data: &[u8]) -> Vec<u8> {
    match variant {
        CompressorVariant::Zlib => {
            // Decompress using Zlib
            let mut decoder = flate2::read::ZlibDecoder::new(data);
            let mut decoded_data = Vec::new();
            decoder
                .read_to_end(&mut decoded_data)
                .expect("Failed to read data from Zlib decoder");
            decoded_data
        }
        CompressorVariant::Gzip => {
            // Decompress using Gzip
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut decoded_data = Vec::new();
            decoder
                .read_to_end(&mut decoded_data)
                .expect("Failed to read data from Gzip decoder");
            decoded_data
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zlib_compression() {
        let data = b"Hello, world!";
        let compressed = compress_data(&CompressorVariant::Zlib, data);
        let decompressed = decompress_data(&CompressorVariant::Zlib, &compressed);
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_gzip_compression() {
        let data = b"Hello, world!";
        let compressed = compress_data(&CompressorVariant::Gzip, data);
        let decompressed = decompress_data(&CompressorVariant::Gzip, &compressed);
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_consistency() {
        let data = b"Hello, world! This is a test of the compression and decompression functions.";
        let compressed_zlib = compress_data(&CompressorVariant::Zlib, data);
        let decompressed_zlib = decompress_data(&CompressorVariant::Zlib, &compressed_zlib);
        assert_eq!(data.to_vec(), decompressed_zlib);
        let compressed_gzip = compress_data(&CompressorVariant::Gzip, data);
        let decompressed_gzip = decompress_data(&CompressorVariant::Gzip, &compressed_gzip);
        assert_eq!(data.to_vec(), decompressed_gzip);
    }
}
