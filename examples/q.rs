use q_compress::{auto_compress, auto_decompress, DEFAULT_COMPRESSION_LEVEL};
use byteorder::{ByteOrder, BigEndian};
use rand::Rng;

const DATALENGTH: usize = 100_000;

// create test data in a vector
fn create_test_data_i64() -> Vec<i64> {
    let mut data : [i64;DATALENGTH] = [1i64;DATALENGTH];

    for i in 0..DATALENGTH {
        data[i] = rand::thread_rng().gen::<i64>();
    }
    data.to_vec()
}

// create test data in a vector
fn create_test_data_f64() -> Vec<f64> {
    let mut data : [f64;DATALENGTH] = [1.0f64;DATALENGTH];

    for i in 0..DATALENGTH {
        data[i] = rand::thread_rng().gen::<f64>();
    }
    data.to_vec()
}

fn example_f64() {
    // // your data
    // let mut my_ints = Vec::new();
    // for i in 0..10000 {
    //   my_ints.push(i as f64);
    // }
    let mut my_ints = create_test_data_f64();

    let u8_len = std::mem::size_of::<f64>() * my_ints.len();
	let mut numbers_u8: Vec<u8> = Vec::with_capacity(u8_len);
    numbers_u8.resize(u8_len, 0u8);
    BigEndian::write_f64_into(&my_ints, &mut numbers_u8);

    println!("orig type: {}", my_ints.len());
    println!("orig byte: {}", numbers_u8.len());
    
    // Here we let the library choose a configuration with default compression
    // level. If you know about the data you're compressing, you can compress
    // faster by creating a `CompressorConfig`.
    let bytes: Vec<u8> = auto_compress(&my_ints, DEFAULT_COMPRESSION_LEVEL);
    
    println!("qco pack   : {} bytes", bytes.len());
   
    // decompress
    let recovered = auto_decompress::<f64>(&bytes).expect("failed to decompress");
    println!("qco unpack : {} elems", recovered.len());

    assert_eq!(recovered, my_ints);

    // lz4

    let mut compressed = Vec::with_capacity(numbers_u8.len());
    let mut unpacked = Vec::with_capacity(numbers_u8.len());

    lz4_compression::compress::compress_into(&numbers_u8, &mut compressed);
    println!("lz4 pack   : {} bytes", compressed.len());

    lz4_compression::decompress::decompress_into(&compressed, &mut unpacked);
    println!("lz4 unpack : {} bytes", unpacked.len());

    assert_eq!(unpacked, numbers_u8);

}

fn example_i64() {
    // // your data
    // let mut my_ints = Vec::new();
    // for i in 0..10000 {
    //   my_ints.push(i as i64);
    // }
    let mut my_ints = create_test_data_i64();

    let u8_len = std::mem::size_of::<i64>() * my_ints.len();
	let mut numbers_u8: Vec<u8> = Vec::with_capacity(u8_len);
    numbers_u8.resize(u8_len, 0u8);
    BigEndian::write_i64_into(&my_ints, &mut numbers_u8);

    println!("orig type: {}", my_ints.len());
    println!("orig byte: {}", numbers_u8.len());
    
    // Here we let the library choose a configuration with default compression
    // level. If you know about the data you're compressing, you can compress
    // faster by creating a `CompressorConfig`.
    let bytes: Vec<u8> = auto_compress(&my_ints, DEFAULT_COMPRESSION_LEVEL);
    
    println!("qco pack   : {} bytes", bytes.len());
   
    // decompress
    let recovered = auto_decompress::<i64>(&bytes).expect("failed to decompress");
    println!("qco unpack : {} elems", recovered.len());

    assert_eq!(recovered, my_ints);

    // lz4

    let mut compressed = Vec::with_capacity(numbers_u8.len());
    let mut unpacked = Vec::with_capacity(numbers_u8.len());

    lz4_compression::compress::compress_into(&numbers_u8, &mut compressed);
    println!("lz4 pack   : {} bytes", compressed.len());

    lz4_compression::decompress::decompress_into(&compressed, &mut unpacked);
    println!("lz4 unpack : {} bytes", unpacked.len());

    assert_eq!(unpacked, numbers_u8);

}

fn main() {
    // example_f64();
    example_i64();
}