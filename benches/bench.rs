use criterion::{Criterion, black_box, criterion_group, criterion_main, PlotConfiguration, AxisScale, Throughput, BenchmarkId, BenchmarkGroup, measurement::WallTime};
use std::io::{Cursor, Read, Write};

extern crate parquet;
use parquet::file::reader::{FileReader, SerializedFileReader};

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use q_compress::{auto_compress, auto_decompress, data_types::NumberLike, DEFAULT_COMPRESSION_LEVEL};
use byteorder::{ByteOrder, BigEndian};

fn extract_column_data_from_parquet(input: &str, column: &str, output: &mut Vec<parquet::record::Field>) -> parquet::basic::Type {

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
	
	let mut p_type = parquet::basic::Type::INT64;
	let mut column_found = false;

	// find data type of the requested column
	for (_pos, col) in fields.iter().enumerate() {	       
		if col.name() == column {
			p_type = col.get_physical_type();
			column_found = true;
			break;
		}		
	} // for each column
	assert_eq!(column_found, true);

	let mut row_iter = reader.get_row_iter(None).unwrap();

	let mut columns : Vec<String> = Vec::new();
	columns.push(String::from(column));

	while let Some(record) = row_iter.next() {	
		// println!("{:?}", record);
		let mut column_iter = record.get_column_iter();
		while let Some(x) = column_iter.next() {
			if columns.contains(x.0) {
				// println!("{:?}", x);
				output.push(x.1.to_owned());
				// match x.1 {
				// 	parquet::record::Field::TimestampMicros(v) => output.push(*v),
				// 	parquet::record::Field::TimestampMillis(v) => output.push(*v),
				// 	parquet::record::Field::Long(v) => output.push(*v),
				// 	_ => {},
				// }
			}
		}
	}
	p_type
}

fn benchmark_qcompress<T: NumberLike>(group: &mut BenchmarkGroup<WallTime>, uncompressed_orig: &Vec<T>, uncompressed_u8: &Vec<u8>, orig_u8_len: usize) {
	
	let mut compressed = Vec::with_capacity(orig_u8_len);

	let mut unpacked_i64: Vec<T> = Vec::with_capacity(uncompressed_orig.len());

	let alg_name = "Q-compression";

	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = uncompressed_orig;
        b.iter(|| {
            black_box(&mut compressed).clear();
			compressed = auto_compress(black_box(orig), DEFAULT_COMPRESSION_LEVEL)
			// compressed = auto_compress(black_box(uncompressed_orig), DEFAULT_COMPRESSION_LEVEL)
        })
    });
    println!("{}: {} {} bytes compression_ratio {:.2}",
		alg_name, uncompressed_u8.len(), compressed.len(),
		uncompressed_u8.len() as f32 / compressed.len() as f32
	);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_i64).clear();
			unpacked_i64 = auto_decompress::<T>(black_box(&compressed)).expect("failed to decompress")
        })
    });
}

fn benchmark_normal_compression(group: &mut BenchmarkGroup<WallTime>, uncompressed_u8: &Vec<u8>, orig_u8_len: usize) {
	let mut compressed = Vec::with_capacity(orig_u8_len);

	let mut unpacked_u8: Vec<u8> = Vec::with_capacity(orig_u8_len);

	let mut alg_name = "";

	// various LZ4 implementtions
	alg_name = "LZ4_compression";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = &uncompressed_u8;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_compression::compress::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lz4_compression::decompress::decompress_into(black_box(&compressed), black_box(&mut unpacked_u8))
        })
    });

	// lz4-flex
	alg_name = "LZ4_flex";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = &uncompressed_u8;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_flex::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lz4_flex::decompress_into(black_box(&compressed), black_box(&mut unpacked_u8))
        })
    });

	alg_name = "LZ4_fear";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let lzfear = lz_fear::CompressionSettings::default();
		let orig = &uncompressed_u8[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzfear.compress(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            let mut lzfear = lz_fear::LZ4FrameReader::new(black_box(&compressed[..])).unwrap().into_read();
            black_box(&mut unpacked_u8).clear();
            lzfear.read_to_end(black_box(&mut unpacked_u8))
        })
    });

    {
		alg_name = "LZZZ_LZ4";
		use lzzzz::lz4::{compress_to_vec, decompress, ACC_LEVEL_DEFAULT};
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
		group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
			let orig = &uncompressed_u8[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				compress_to_vec(black_box(orig), black_box(&mut compressed), ACC_LEVEL_DEFAULT)
			})
		});

		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
			b.iter(|| {
				black_box(&mut unpacked_u8).clear();
				decompress(black_box(&compressed), black_box(&mut unpacked_u8))
			})
		});

		alg_name = "LZZZ_LZ4HC";
		use lzzzz::lz4_hc::{compress_to_vec as lz4_hc_compress_to_vec, CLEVEL_DEFAULT};
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
        group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
            let orig = &uncompressed_u8[..];
            b.iter(|| {
                black_box(&mut compressed).clear();
                lz4_hc_compress_to_vec(black_box(orig), black_box(&mut compressed), CLEVEL_DEFAULT)
            })
        });

        println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);
		
        // Same as lzzzz/lz4.unpack (just for normalized output / reports)
		group.throughput(Throughput::Elements(compressed.len() as u64));
        group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
            b.iter(|| {
                black_box(&mut unpacked_u8).clear();
                decompress(black_box(&compressed), black_box(&mut unpacked_u8))
            })
        });

		alg_name = "LZZZ_LZ4F";
        use lzzzz::lz4f::{
            compress_to_vec as lz4f_compress_to_vec, decompress_to_vec as lz4f_decompress_to_vec,
            Preferences,
        };
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
        group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
            let orig = &uncompressed_u8[..];
            b.iter(|| {
                black_box(&mut compressed).clear();
                lz4f_compress_to_vec(black_box(orig), black_box(&mut compressed), &Preferences::default())
            })
        });
        
		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
        group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
            b.iter(|| {
                black_box(&mut unpacked_u8).clear();
                lz4f_decompress_to_vec(black_box(&compressed), black_box(&mut unpacked_u8))
            })
        });
	}
	// zstandard
	for level in 1..10 {
		let alg_name = &format!("ZSTD-{:?}", level);
		
		// first, obtain comp_len
		let orig = &uncompressed_u8;
		compressed = zstd::bulk::compress(orig, level).unwrap();
		
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
		group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
			let orig = &uncompressed_u8;
			b.iter(|| {
				compressed = zstd::bulk::compress(black_box(orig), black_box(level)).unwrap();
			})
		});
		
		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
			b.iter(|| {
				black_box(&mut unpacked_u8).clear();
				unpacked_u8 = zstd::bulk::decompress( black_box(&mut compressed), orig_u8_len).unwrap();
			})
		});
	}
	// snappy
	alg_name = "SNAPPY";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
	group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = &uncompressed_u8;
        b.iter(|| {
			black_box(&mut compressed).clear();
			compressed = snap::raw::Encoder::new().compress_vec(black_box(orig)).unwrap();
        })
    });

    println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
			black_box(&mut unpacked_u8).clear();
			unpacked_u8 = snap::raw::Decoder::new().decompress_vec(black_box(&mut compressed)).unwrap();
        })
    });
	/*
    group.bench_function("snappy-framed.pack", |b| {
        b.iter(|| {
			black_box(&mut compressed).clear();
			let mut enc = snappy_framed::write::SnappyFramedEncoder::new(black_box(&mut compressed)).unwrap();
			enc.write_all(black_box(orig)).unwrap();
			enc.flush().unwrap();
        })
    });
    println!("snappy-framed: {} bytes", compressed.len());
    group.bench_function("snappy-framed.unpack.nocrc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(black_box(&compressed));
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Ignore);
			decoder.read_to_end(black_box(&mut unpacked))
        })
    });
    group.bench_function("snappy-framed.unpack.crc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(&compressed);
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Verify);
            decoder.read_to_end(black_box(&mut unpacked))
        })
    });
	*/
    // deflate
    use deflate::Compression;
    for &level in &[Compression::Fast, Compression::Default, Compression::Best] {
		let alg_name = &format!("DEFLATE-{:?}", level);
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
		group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
			let orig = &uncompressed_u8;
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = deflate::write::DeflateEncoder::new(black_box(std::mem::replace(&mut compressed, Vec::new())), level);
				encoder.write_all(black_box(orig)).unwrap();
				compressed = black_box(encoder.finish().unwrap());
			})
		});

		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
			b.iter(|| {
				deflate::deflate_bytes(black_box(&compressed))
			})
		});
	}

	// flate
	// flate requires caller to set the correct size of the output vec, otherwise it refuses to work properly
	for level in flate2::Compression::fast().level() .. flate2::Compression::best().level() {

		let alg_name = &format!("FLATE-{:?}", level);

		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
		group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
			let orig = &uncompressed_u8;
			compressed.clear();
			compressed.resize(orig_u8_len * 2, 0_u8);
			b.iter(|| {
				let mut encoder = flate2::Compress::new(flate2::Compression::new(level), false);
				let _res = encoder.compress(
					black_box(orig), 
					black_box(&mut compressed),
					flate2::FlushCompress::Finish
				).unwrap();
			})
		});

		// obtain total size of compression
		let orig = &uncompressed_u8;
		compressed.clear();
		compressed.resize(orig_u8_len * 2, 0_u8);
		let mut encoder = flate2::Compress::new(flate2::Compression::new(level), false);
		let _res = encoder.compress(black_box(orig), black_box(&mut compressed),flate2::FlushCompress::Finish).unwrap();
		
		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, encoder.total_in(), encoder.total_out(), 
		encoder.total_in() as f32 / encoder.total_out() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
			unpacked_u8.clear();
			unpacked_u8.resize(orig_u8_len, 0_u8);
			b.iter(|| {
				let mut decoder = flate2::Decompress::new(false);
        		let _res = decoder.decompress(
                        black_box(&compressed),
                        black_box(&mut unpacked_u8),
                        flate2::FlushDecompress::Finish
                    );
			})
		});
	}

	// YAZI
	use yazi::CompressionLevel;
	for &level in &[CompressionLevel::BestSpeed, CompressionLevel::Default, CompressionLevel::BestSize] {
		
		let alg_name = &format!("YAZI-{:?}", level);
		
		group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
		group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
			let orig = &uncompressed_u8[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = yazi::Encoder::new();
				encoder.set_format(yazi::Format::Raw);
				encoder.set_level(level);
				let mut stream = encoder.stream_into_vec(black_box(&mut compressed));
				stream.write(black_box(orig)).unwrap();
				stream.finish()
			})
		});
		
		println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
			b.iter(|| {
				black_box(&mut unpacked_u8).clear();
				let mut decoder = yazi::Decoder::new();
				decoder.set_format(yazi::Format::Raw);
				let mut stream = decoder.stream_into_vec(&mut unpacked_u8);
				stream.write(black_box(&compressed)).unwrap();
				stream.finish()
			})
		});
	}

	// LZMA
	alg_name = "LZMA";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
	group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		// let orig = &mut &uncompressed_u8[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma_compress(black_box(&mut &uncompressed_u8[..]), black_box(&mut compressed))
        })
    });

    println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lzma_rs::lzma_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8))
        })
    });

	alg_name = "LZMA2";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		// let orig = &mut &uncompressed_u8[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma2_compress(black_box(&mut &uncompressed_u8[..]), black_box(&mut compressed))
        })
    });
    
	println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lzma_rs::lzma2_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8))
        })
    });

	alg_name = "LZMA_XZ";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		// let orig = &mut &uncompressed_u8[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::xz_compress(black_box(&mut &uncompressed_u8[..]), black_box(&mut compressed))
        })
    });
    
	println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lzma_rs::xz_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8))
        })
    });

	/*
    // Zopfli
	alg_name = "ZOPFLI";
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
	group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            zopfli::compress(
				&zopfli::Options::default(),
				&zopfli::Format::Deflate,
				black_box(&mut &uncompressed[..]),
				black_box(&mut compressed)
			).unwrap()
        })
    });
    println!("zopfli: {} bytes", compressed.len());

    // no decompression here

	*/

    // Brotli
	alg_name = "BROTLI";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
    group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = &mut &uncompressed_u8[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            brotli::BrotliCompress(
				black_box(orig),
				black_box(&mut compressed),
				&Default::default(),
			)
        })
    });
    
	println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            brotli::BrotliDecompress(black_box(&mut &compressed[..]), black_box(&mut unpacked_u8))
        })
    });

	/* disable tar, because no unpacking
    // tar
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
    group.bench_function(BenchmarkId::new("pack", "tar"), |b| {
		compressed.reserve(uncompressed.len()); // double the excitement! ;P
        b.iter(|| {
			black_box(&mut compressed).clear();
			use tar::{Builder, Header};

			let mut header = Header::new_gnu();
			header.set_path("data").unwrap();
			header.set_size(4);
			header.set_cksum();

			let mut ar = Builder::new(std::mem::replace(&mut compressed, Vec::new()));
			ar.append(&header, &orig[..]).unwrap();
			compressed = ar.into_inner().unwrap();
			compressed.len()
		})
	});
	println!("tar: {} bytes", compressed.len());
	// unfortunately, tar only unpacks to disk, so this result would be
	// incomparable. Perhaps in a later benchmark.

	*/
	
    // zip
	alg_name = "ZIP";
	group.throughput(Throughput::Elements(uncompressed_u8.len() as u64));
	group.bench_function(BenchmarkId::new("pack", alg_name), |b| {
		let orig = &uncompressed_u8;
        b.iter(|| {
			black_box(&mut compressed).clear();
			let mut zipw = zip::ZipWriter::new(Cursor::new(black_box(Vec::new())));
			let options = zip::write::FileOptions::default();
        			// .compression_method(zip::CompressionMethod::Stored)
        			// .unix_permissions(0o755);
			zipw.start_file("data", options).unwrap();
			zipw.write_all(black_box(orig)).unwrap();
			let c = zipw.finish().unwrap();
			compressed = c.into_inner();
		})
	});
	
	println!("{}: {} {} bytes compression_ratio {:.2}", alg_name, uncompressed_u8.len(), compressed.len(), 
		orig_u8_len as f32 / compressed.len() as f32);

	group.throughput(Throughput::Elements(compressed.len() as u64));
	group.bench_function(BenchmarkId::new("unpack", alg_name), |b| {
		b.iter(|| {
			black_box(&mut unpacked_u8).clear();
			let mut zipr = zip::ZipArchive::new(Cursor::new(black_box(&compressed))).unwrap();
			zipr.by_index(0).unwrap().read_to_end(black_box(&mut unpacked_u8)).unwrap();
		})
	});

	
}

fn compare_compress_i64(c: &mut Criterion) {
	// two benchmark groups
    let mut group = c.benchmark_group("compression");

	// plot config
	let plot_config_compress = PlotConfiguration::default()
													.summary_scale(AxisScale::Logarithmic);

    group.plot_config(plot_config_compress);

	// prepare raw data, load into memory

	// let uncompressed_u8 = std::fs::read(
	// 	std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS")
	// ).expect("reading $FILE_TO_COMPRESS");

	// // q-compress
	// let i64_len = uncompressed_u8 / std::mem::size_of::<i64>();
	// let mut numbers_got: Vec<i64> = Vec::with_capacity(i64_len);
    // numbers_got.resize(i64_len, 0i64);

	// BigEndian::read_i64_into(&uncompressed_u8, &mut numbers_got);

	let mut uncompressed_field: Vec<parquet::record::Field> = Vec::new();

	let element_type = extract_column_data_from_parquet(
		// parquet file
		&std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS"), 
		// column in parquet
		&std::env::var("PARQUET_COLUMN").expect("set $PARQUET_COLUMN"),
		// store data in a vec 
		&mut uncompressed_field);
	
	assert!(uncompressed_field.len() >= 1);

	let first_element = &uncompressed_field[0];

	println!("data type: {:?}", first_element);

	match first_element {
		parquet::record::Field::TimestampMicros(_)
		| parquet::record::Field::TimestampMillis(_) 
		| parquet::record::Field::Long(_) 
		| parquet::record::Field::ULong(_) => {
			// i64

			let mut uncompressed_orig = Vec::new();

			// collect data
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::TimestampMicros(v)
					| parquet::record::Field::TimestampMillis(v) 
					| parquet::record::Field::Long(v) => {
						uncompressed_orig.push(*v);
					},
					parquet::record::Field::ULong(v) => {
						uncompressed_orig.push(*v as i64);
					},
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_i64_len = uncompressed_orig.len();
			let orig_u8_len = orig_i64_len * std::mem::size_of::<i64>();

			println!("uncompressed_orig: {} items {} u8int", orig_i64_len, orig_u8_len);

			// convert to u8
			let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
			uncompressed_u8.resize(orig_u8_len, 0u8);
			BigEndian::write_i64_into(&uncompressed_orig[..], &mut uncompressed_u8);
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress
			benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			// benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();
		},

		parquet::record::Field::Double(_) => {
			// i32, u32

			let mut uncompressed_orig = Vec::new();

			// collect data
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::Double(v) => {
						uncompressed_orig.push(*v);
					},
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_f64_len = uncompressed_orig.len();
			let orig_u8_len = orig_f64_len * std::mem::size_of::<f64>();

			println!("uncompressed_orig: {} items {} u8int", orig_f64_len, orig_u8_len);

			// convert to u8
			let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
			uncompressed_u8.resize(orig_u8_len, 0u8);
			BigEndian::write_f64_into(&uncompressed_orig[..], &mut uncompressed_u8);
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress
			benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			// benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();
		},

		parquet::record::Field::Int(_)
		| parquet::record::Field::Date(_)
		| parquet::record::Field::UInt(_)  => {
			// i32, u32

			let mut uncompressed_orig = Vec::new();

			// collect data
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::Int(v)
					| parquet::record::Field::Date(v) => {
						uncompressed_orig.push(*v);
					},
					parquet::record::Field::UInt(v) => {
						uncompressed_orig.push(*v as i32);
					},
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_i32_len = uncompressed_orig.len();
			let orig_u8_len = orig_i32_len * std::mem::size_of::<i32>();

			println!("uncompressed_orig: {} items {} u8int", orig_i32_len, orig_u8_len);

			// convert to u8
			let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
			uncompressed_u8.resize(orig_u8_len, 0u8);
			BigEndian::write_i32_into(&uncompressed_orig[..], &mut uncompressed_u8);
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress
			benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			// benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();
		},

		parquet::record::Field::Float(_) => {
			// i32, u32

			let mut uncompressed_orig = Vec::new();

			// collect data
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::Float(v) => {
						uncompressed_orig.push(*v);
					},
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_f32_len = uncompressed_orig.len();
			let orig_u8_len = orig_f32_len * std::mem::size_of::<f32>();

			println!("uncompressed_orig: {} items {} u8int", orig_f32_len, orig_u8_len);

			// convert to u8
			let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
			uncompressed_u8.resize(orig_u8_len, 0u8);
			BigEndian::write_f32_into(&uncompressed_orig[..], &mut uncompressed_u8);
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress
			benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			// benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();
		},

		parquet::record::Field::Short(_)
		| parquet::record::Field::UShort(_) => {
			// i16, u16

			let mut uncompressed_orig = Vec::new();

			// collect data
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::Short(v) => {
						uncompressed_orig.push(*v);
					},
					parquet::record::Field::UShort(v) => {
						uncompressed_orig.push(*v as i16);
					},
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_i16_len = uncompressed_orig.len();
			let orig_u8_len = orig_i16_len * std::mem::size_of::<i16>();

			println!("uncompressed_orig: {} items {} u8int", orig_i16_len, orig_u8_len);

			// convert to u8
			let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
			uncompressed_u8.resize(orig_u8_len, 0u8);
			BigEndian::write_i16_into(&uncompressed_orig[..], &mut uncompressed_u8);
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress
			benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			// benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();
		},

		parquet::record::Field::Str(_)
		| parquet::record::Field::Bytes(_) => {
			let mut uncompressed_orig = Vec::new();
			for i in 0 .. uncompressed_field.len()  {
				let x = &uncompressed_field[i];
				match x {
					parquet::record::Field::Str(v) => {
						uncompressed_orig.append(&mut v.as_bytes().to_vec());
					},
					parquet::record::Field::Bytes(v) => {
						uncompressed_orig.append(&mut v.data().to_vec());
					}
					_ => {
						panic!("Error: column type mismatch. i {} x {:?} element_type {:?}", i, x, element_type);
					}
				}
			}

			// set up data structures
			let orig_u8_len = uncompressed_orig.len();

			println!("uncompressed_orig: {} items {} u8int", orig_u8_len, orig_u8_len);

			// convert to u8
			let uncompressed_u8 = uncompressed_orig;
			
			println!("uncompressed_u8: {} items {} u8int", uncompressed_u8.len(), 
				uncompressed_u8.len() * std::mem::size_of::<u8>());

			// run q-compress : does not support Vec<u8> yet
			// benchmark_qcompress(&mut group, &uncompressed_orig, &uncompressed_u8, orig_u8_len);

			// run normal compressors
			benchmark_normal_compression(&mut group, &uncompressed_u8, orig_u8_len);

			group.finish();

		},
		
		_ => {
			panic!("Error: element_type {:?} has not been implemented yet", element_type);
		},
	}
	
	// println!("Obtained data: {:?}", uncompressed_orig);

}

fn compare_compress(c: &mut Criterion) {
	// two benchmark groups
    let mut group = c.benchmark_group("compression");

	// plot config
	let plot_config_compress = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    group.plot_config(plot_config_compress);

	// prepare raw data, load into memory
	let uncompressed = std::fs::read(
		std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS")
	).expect("reading $FILE_TO_COMPRESS");
	let orig_len = uncompressed.len();
	
	// prepare data structures for benchmark
	println!("uncompressed: {} bytes", orig_len);
	let mut compressed = Vec::with_capacity(orig_len);
	let mut unpacked = Vec::with_capacity(orig_len);

	// various LZ4 implementtions
	group.throughput(Throughput::Elements(orig_len as u64));
    group.bench_function(BenchmarkId::new("pack", "lz4"), |b| {
		let orig = &uncompressed;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_compression::compress::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz4-compression: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lz4"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lz4_compression::decompress::decompress_into(black_box(&compressed), black_box(&mut unpacked))
        })
    });
	group.throughput(Throughput::Elements(orig_len as u64));
    group.bench_function(BenchmarkId::new("pack", "lz4_flex"), |b| {
		let orig = &uncompressed;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_flex::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz4_flex: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lz4_flex"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lz4_flex::decompress_into(black_box(&compressed), black_box(&mut unpacked))
        })
    });
	group.throughput(Throughput::Elements(orig_len as u64));
    group.bench_function(BenchmarkId::new("pack", "lz4_fear"), |b| {
		let lzfear = lz_fear::CompressionSettings::default();
		let orig = &uncompressed[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzfear.compress(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz_fear: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lz4_fear"), |b| {
        b.iter(|| {
            let mut lzfear = lz_fear::LZ4FrameReader::new(black_box(&compressed[..])).unwrap().into_read();
            black_box(&mut unpacked).clear();
            lzfear.read_to_end(black_box(&mut unpacked))
        })
    });

    {
		use lzzzz::lz4::{compress_to_vec, decompress, ACC_LEVEL_DEFAULT};
		group.throughput(Throughput::Elements(orig_len as u64));
		group.bench_function(BenchmarkId::new("pack", "lzzzz-lz4"), |b| {
			let orig = &uncompressed[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				compress_to_vec(black_box(orig), black_box(&mut compressed), ACC_LEVEL_DEFAULT)
			})
		});

		println!("lzzzz: {} bytes", compressed.len());
		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", "lzzzz-lz4"), |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				decompress(black_box(&compressed), black_box(&mut unpacked))
			})
		});

		use lzzzz::lz4_hc::{compress_to_vec as lz4_hc_compress_to_vec, CLEVEL_DEFAULT};
		group.throughput(Throughput::Elements(orig_len as u64));
        group.bench_function(BenchmarkId::new("pack", "lzzzz-lz4-hc"), |b| {
            let orig = &uncompressed[..];
            b.iter(|| {
                black_box(&mut compressed).clear();
                lz4_hc_compress_to_vec(black_box(orig), black_box(&mut compressed), CLEVEL_DEFAULT)
            })
        });
        println!("lzzzz/lz4_hc: {} bytes", compressed.len());
		
        // Same as lzzzz/lz4.unpack (just for normalized output / reports)
		group.throughput(Throughput::Elements(compressed.len() as u64));
        group.bench_function(BenchmarkId::new("unpack", "lzzzz-lz4-hc"), |b| {
            b.iter(|| {
                black_box(&mut unpacked).clear();
                decompress(black_box(&compressed), black_box(&mut unpacked))
            })
        });

        use lzzzz::lz4f::{
            compress_to_vec as lz4f_compress_to_vec, decompress_to_vec as lz4f_decompress_to_vec,
            Preferences,
        };
		group.throughput(Throughput::Elements(orig_len as u64));
        group.bench_function(BenchmarkId::new("pack", "lzzzz-lz4f"), |b| {
            let orig = &uncompressed[..];
            b.iter(|| {
                black_box(&mut compressed).clear();
                lz4f_compress_to_vec(
                    black_box(orig),
                    black_box(&mut compressed),
                    &Preferences::default(),
                )
            })
        });
        println!("lzzzz/lz4f: {} bytes", compressed.len());

		group.throughput(Throughput::Elements(compressed.len() as u64));
        group.bench_function(BenchmarkId::new("unpack", "lzzzz-lz4f"), |b| {
            b.iter(|| {
                black_box(&mut unpacked).clear();
                lz4f_decompress_to_vec(black_box(&compressed), black_box(&mut unpacked))
            })
        });
	}
	// zstandard
	for level in 1..10 {
		let orig = &uncompressed;
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
		let comp_len = zstd::bulk::compress_to_buffer(orig, &mut compressed, level).unwrap();

		group.throughput(Throughput::Elements(orig_len as u64));
		group.bench_function(BenchmarkId::new("pack", format!("zstd-{:?}", level)), |b| {
			b.iter(|| {
				zstd::bulk::compress_to_buffer(black_box(orig), black_box(&mut compressed), black_box(level))
			})
		});
		println!("zstd-{}: {} bytes", level, comp_len);
		unpacked.clear();
		unpacked.resize(orig_len, 0_u8);
		let comp = &compressed[..comp_len];
		group.throughput(Throughput::Elements(comp.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", format!("zstd-{:?}", level)), |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				zstd::bulk::decompress_to_buffer(black_box(comp), black_box(&mut unpacked))
			})
		});
	}
	// snappy
	let orig = &uncompressed;
	group.throughput(Throughput::Elements(orig_len as u64));
	group.bench_function(BenchmarkId::new("pack", "snap"), |b| {
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
        b.iter(|| {
			snap::raw::Encoder::new().compress(black_box(orig), black_box(&mut compressed))
        })
    });
    let comp_len = snap::raw::Encoder::new().compress(orig, &mut compressed).unwrap();
    println!("snap: {} bytes", comp_len);

	group.throughput(Throughput::Elements(comp_len as u64));
    group.bench_function(BenchmarkId::new("unpack", "snap"), |b| {
		unpacked.clear();
		unpacked.resize(orig_len, 0_u8);
		let comp = &compressed[..comp_len];
        b.iter(|| {
            snap::raw::Decoder::new().decompress(black_box(comp), black_box(&mut unpacked))
        })
    });
	/*
    group.bench_function("snappy-framed.pack", |b| {
        b.iter(|| {
			black_box(&mut compressed).clear();
			let mut enc = snappy_framed::write::SnappyFramedEncoder::new(black_box(&mut compressed)).unwrap();
			enc.write_all(black_box(orig)).unwrap();
			enc.flush().unwrap();
        })
    });
    println!("snappy-framed: {} bytes", compressed.len());
    group.bench_function("snappy-framed.unpack.nocrc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(black_box(&compressed));
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Ignore);
			decoder.read_to_end(black_box(&mut unpacked))
        })
    });
    group.bench_function("snappy-framed.unpack.crc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(&compressed);
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Verify);
            decoder.read_to_end(black_box(&mut unpacked))
        })
    });
	*/
    // deflate
    use deflate::Compression;
    for &level in &[Compression::Fast, Compression::Default, Compression::Best] {
		group.throughput(Throughput::Elements(orig_len as u64));
		group.bench_function(BenchmarkId::new("pack", format!("deflate-{:?}", level)), |b| {
			let orig = &uncompressed;
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = deflate::write::DeflateEncoder::new(black_box(std::mem::replace(&mut compressed, Vec::new())), level);
				encoder.write_all(black_box(orig)).unwrap();
				compressed = black_box(encoder.finish().unwrap());
			})
		});
		println!("deflate-{:?}: {} bytes", level, compressed.len());
		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", format!("deflate-{:?}", level)), |b| {
			b.iter(|| {
				deflate::deflate_bytes(black_box(&compressed))
			})
		});
	}
	for level in flate2::Compression::fast().level() .. flate2::Compression::best().level() {
		group.throughput(Throughput::Elements(orig_len as u64));
		group.bench_function(BenchmarkId::new("pack", format!("flate2-{:?}", level)), |b| {
			compressed.clear();
			compressed.resize(orig_len * 2, 0_u8);
			b.iter(|| {
				flate2::Compress::new(flate2::Compression::new(level), false).compress(
					black_box(orig), 
					black_box(&mut compressed),
					flate2::FlushCompress::Finish
				)
			})
		});
		let mut comp = flate2::Compress::new(flate2::Compression::new(level), false);
		comp.compress(
			black_box(orig), 
			black_box(&mut compressed),
			flate2::FlushCompress::Finish
		).unwrap();
		println!("flate2-{}: {} bytes", level, comp.total_out());
		group.throughput(Throughput::Elements(comp_len as u64));
		group.bench_function(BenchmarkId::new("unpack", format!("flate2-{:?}", level)), |b| {
			unpacked.clear();
			unpacked.resize(orig_len, 0_u8);
			let comp = &compressed[..comp_len];
			b.iter(|| {
				flate2::Decompress::new(false).decompress(
					black_box(comp),
					black_box(&mut unpacked),
					flate2::FlushDecompress::Finish
				)
			})
		});
	}
	use yazi::CompressionLevel;
	for &level in &[CompressionLevel::BestSpeed, CompressionLevel::Default, CompressionLevel::BestSize] {
		group.throughput(Throughput::Elements(orig_len as u64));
		group.bench_function(BenchmarkId::new("pack", format!("yazi-{:?}", level)), |b| {
			let orig = &uncompressed[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = yazi::Encoder::new();
				encoder.set_format(yazi::Format::Raw);
				encoder.set_level(level);
				let mut stream = encoder.stream_into_vec(black_box(&mut compressed));
				stream.write(black_box(orig)).unwrap();
				stream.finish()
			})
		});
		println!("yazi-{:?}: {} bytes", level, compressed.len());
		group.throughput(Throughput::Elements(compressed.len() as u64));
		group.bench_function(BenchmarkId::new("unpack", format!("yazi-{:?}", level)), |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				let mut decoder = yazi::Decoder::new();
				decoder.set_format(yazi::Format::Raw);
				let mut stream = decoder.stream_into_vec(&mut unpacked);
				stream.write(black_box(&compressed)).unwrap();
				stream.finish()
			})
		});
	}
	// LZMA
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
	group.bench_function(BenchmarkId::new("pack", "lzma"), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma-rs: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lzma"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::lzma_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });

	group.throughput(Throughput::Elements(uncompressed.len() as u64));
    group.bench_function(BenchmarkId::new("pack", "lzma2"), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma2_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma2: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lzma2"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::lzma2_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });

	group.throughput(Throughput::Elements(uncompressed.len() as u64));
    group.bench_function(BenchmarkId::new("pack", "lzma_xz"), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::xz_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma-rs/xz: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lzma_xz"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::xz_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });

    // Zopfli
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
	group.bench_function(BenchmarkId::new("pack", "zopfli"), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            zopfli::compress(
				&zopfli::Options::default(),
				&zopfli::Format::Deflate,
				black_box(&mut &uncompressed[..]),
				black_box(&mut compressed)
			).unwrap()
        })
    });
    println!("zopfli: {} bytes", compressed.len());
    // no decompression here

    // Brotli
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
    group.bench_function(BenchmarkId::new("pack", "brotli"), |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            brotli::BrotliCompress(
				black_box(&mut &uncompressed[..]),
				black_box(&mut compressed),
				&Default::default(),
			)
        })
    });
    println!("brotli: {} bytes", compressed.len());
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "brotli"), |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            brotli::BrotliDecompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });

	/* disable tar, because no unpacking
    // tar
	group.throughput(Throughput::Elements(uncompressed.len() as u64));
    group.bench_function(BenchmarkId::new("pack", "tar"), |b| {
		compressed.reserve(uncompressed.len()); // double the excitement! ;P
        b.iter(|| {
			black_box(&mut compressed).clear();
			use tar::{Builder, Header};

			let mut header = Header::new_gnu();
			header.set_path("data").unwrap();
			header.set_size(4);
			header.set_cksum();

			let mut ar = Builder::new(std::mem::replace(&mut compressed, Vec::new()));
			ar.append(&header, &orig[..]).unwrap();
			compressed = ar.into_inner().unwrap();
			compressed.len()
		})
	});
	println!("tar: {} bytes", compressed.len());
	// unfortunately, tar only unpacks to disk, so this result would be
	// incomparable. Perhaps in a later benchmark.

	*/
	
    // zip
	group.throughput(Throughput::Elements(orig_len as u64));
	group.bench_function(BenchmarkId::new("pack", "zip"), |b| {
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
        b.iter(|| {
			let mut zipw = zip::ZipWriter::new(Cursor::new(black_box(&mut compressed)));
			zipw.start_file("data", zip::write::FileOptions::default()).unwrap();
			zipw.write(black_box(orig)).unwrap();
			zipw.finish().unwrap();
		})
	});
	let ziplen = {
		let mut zipw = zip::ZipWriter::new(Cursor::new(&mut compressed));
		zipw.start_file("data", zip::write::FileOptions::default()).unwrap();
		zipw.write(orig).unwrap();
		let c = zipw.finish().unwrap();
		c.position() as usize
	};
	println!("zip: {} bytes", ziplen);
	group.throughput(Throughput::Elements(ziplen as u64));
	group.bench_function(BenchmarkId::new("unpack", "zip"), |b| {
		b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut zipr = zip::ZipArchive::new(Cursor::new(black_box(&compressed[..ziplen]))).unwrap();
			zipr.by_index(0).unwrap().read_to_end(black_box(&mut unpacked)).unwrap();
		})
	});

    group.finish();
	
}

criterion_group!(
	name = benches;
	config = Criterion::default().sample_size(10);
	targets = compare_compress_i64);
criterion_main!(benches);
