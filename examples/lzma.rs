extern crate snap;

use criterion::black_box;
use std::io::{Cursor, Read, Write};

extern crate parquet;
use parquet::file::reader::{FileReader, SerializedFileReader};

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use byteorder::{ByteOrder, BigEndian};

fn extract_column_data_from_parquet(input: &str, column: &str, output: &mut Vec<i64>) {

	println!("input {:?}, column {:?}", input, column);

	let parquet_path = input;	
   	let file = File::open( Path::new(parquet_path) )
       .expect("Couldn't open parquet data");
       
	let reader:SerializedFileReader<File> = SerializedFileReader::new(file).unwrap();
	let parquet_metadata = reader.metadata();               
	
	// Writing the type signature here, to be super 
	// clear about the return type of get_fields()
	let fields:&[Arc<parquet::schema::types::Type>] = parquet_metadata
		.file_metadata()
		.schema()
		.get_fields(); 

	let mut row_iter = reader.get_row_iter(None).unwrap();

	let mut columns : Vec<String> = Vec::new();
	columns.push(String::from(column));

	while let Some(record) = row_iter.next() {	
		// println!("{:?}", record);
		let mut column_iter = record.get_column_iter();
		while let Some(x) = column_iter.next() {
			if columns.contains(x.0) {
				// println!("{:?}", x);
				match x.1 {
					parquet::record::Field::TimestampMicros(v) => output.push(*v),
					parquet::record::Field::TimestampMillis(v) => output.push(*v),
					parquet::record::Field::Long(v) => output.push(*v),
					_ => {},
				}
			}
		}
	}
}


fn main() {

    let mut uncompressed_i64: Vec<i64> = Vec::new();

	extract_column_data_from_parquet(
        &std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS"), 
		"timestamp", 
		&mut uncompressed_i64);

    let orig_i64_len = uncompressed_i64.len();
    let orig_u8_len = orig_i64_len * std::mem::size_of::<i64>();
    
    println!("uncompressed_i64: {} items {} bytes", orig_i64_len, orig_u8_len);
    
    let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
    uncompressed_u8.resize(orig_u8_len, 0u8);
    BigEndian::write_i64_into(&uncompressed_i64, &mut uncompressed_u8);

    // compressed.resize(uncompressed.len() * 2, 0_u8);
    let mut compressed = Vec::new();

    let orig = &uncompressed_u8;
    let level = 9;

    
    let mut unpacked_u8: Vec<u8> = Vec::with_capacity(orig_u8_len);

    lzma_rs::lzma_compress(black_box(&mut &uncompressed_u8[..]), black_box(&mut compressed));
    
    println!("{}", compressed.len());

    lzma_rs::lzma_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8));
    
    println!("{}", unpacked_u8.len());
    assert_eq!(uncompressed_u8, unpacked_u8);

    lzma_rs::lzma2_compress(black_box(&mut &uncompressed_u8[..]), black_box(&mut compressed));
    
    println!("{}", compressed.len());

    lzma_rs::lzma2_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8));

    println!("{}", unpacked_u8.len());
    assert_eq!(uncompressed_u8, unpacked_u8);
}