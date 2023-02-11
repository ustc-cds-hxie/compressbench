extern crate snap;

use criterion::black_box;

fn main() {
    let uncompressed = vec![1, 2, 3, 4, 5, 6, 7];
    // let mut compressed : Vec<u8> = Vec::new();

    let orig_u8_len = uncompressed.len();

    // compressed.resize(uncompressed.len() * 2, 0_u8);
    let mut compressed = Vec::new();

    let orig = &uncompressed;
    let level = 9;
    
    let mut unpacked_u8: Vec<u8> = Vec::with_capacity(orig_u8_len);

    for level in flate2::Compression::fast().level() .. flate2::Compression::best().level() {
        
        println!("level {}", level);

        compressed.clear();
        compressed.resize(orig_u8_len * 2, 0_u8);

        unpacked_u8.clear();
        unpacked_u8.resize(orig_u8_len * 2, 0_u8);

        let mut encoder = flate2::Compress::new(flate2::Compression::new(level), false);
        let res = encoder.compress(
            black_box(orig), 
            black_box(&mut compressed),
            flate2::FlushCompress::Finish
        ).unwrap();

        println!("encoder in {} out {} res {:?}", encoder.total_in(), encoder.total_out(), res);

        let mut decoder = flate2::Decompress::new(false);
        let res = decoder.decompress(
                        black_box(&compressed),
                        black_box(&mut unpacked_u8),
                        flate2::FlushDecompress::Finish
                    );

        println!("decoder in {} out {} res {:?}", decoder.total_in(), decoder.total_out(), res);

        println!("{} {:x?}", compressed.len(), compressed);
        println!("{} {:x?}", unpacked_u8.len(), unpacked_u8);
    }


}