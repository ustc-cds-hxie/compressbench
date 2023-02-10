extern crate zip;

use std::io::{Cursor, Read, Write};

fn main() {
    let uncompressed = vec![1, 2, 3, 4, 5, 6, 7];
    // let mut compressed : Vec<u8> = Vec::new();

    // compressed.resize(uncompressed.len() * 2, 0_u8);
    let mut compressed = Vec::new();

    let orig = &uncompressed;

    let mut zipw = zip::ZipWriter::new(Cursor::new(Vec::new()));
	
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    zipw.start_file("data", options).unwrap();
	
    zipw.write_all(orig).unwrap();
	
    let c = zipw.finish().unwrap();

    // let uncompressed = zstd::bulk::decompress(&compressed, uncompressed.len()).unwrap();

    compressed = c.into_inner();

    println!("{} {:x?}", compressed.len(), compressed);
    println!("{} {:x?}", uncompressed.len(), uncompressed);
}

