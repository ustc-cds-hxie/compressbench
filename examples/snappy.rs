extern crate snap;

use criterion::black_box;

fn main() {
    let uncompressed = vec![1, 2, 3, 4, 5, 6, 7];
    // let mut compressed : Vec<u8> = Vec::new();

    let orig_u8_len = uncompressed.len();

    // compressed.resize(uncompressed.len() * 2, 0_u8);
    let mut compressed = Vec::new();
    compressed.resize(orig_u8_len * 2, 0_u8);

    println!("{}", compressed.len());

    let orig = &uncompressed;
    let level = 9;

    
    let mut unpacked_u8: Vec<u8> = Vec::with_capacity(orig_u8_len);

    compressed = snap::raw::Encoder::new().compress_vec(black_box(orig)).unwrap();

    unpacked_u8 = snap::raw::Decoder::new().decompress_vec(black_box(&mut compressed)).unwrap();
    
    println!("{} {:x?}", compressed.len(), compressed);
    println!("{} {:x?}", unpacked_u8.len(), unpacked_u8);
}