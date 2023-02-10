extern crate zstd;

fn main() {
    let uncompressed = vec![1, 2, 3, 4, 5, 6, 7];
    // let mut compressed : Vec<u8> = Vec::new();

    // compressed.resize(uncompressed.len() * 2, 0_u8);
    let mut compressed = Vec::new();

    println!("{}", compressed.len());

    let orig = &uncompressed;
    let level = 9;

	// let comp_len = zstd::bulk::compress_to_buffer(orig, &mut compressed, level).unwrap();

    compressed = zstd::bulk::compress(orig, level).unwrap();

    let uncompressed = zstd::bulk::decompress(&compressed, uncompressed.len()).unwrap();

    println!("{} {:x?}", compressed.len(), compressed);
    println!("{} {:x?}", uncompressed.len(), uncompressed);
}