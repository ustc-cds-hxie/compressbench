use criterion::{Criterion, black_box, criterion_group, criterion_main, PlotConfiguration, AxisScale, Throughput, BenchmarkId};
use std::io::{Cursor, Read, Write};

extern crate parquet;
use parquet::file::reader::{FileReader, SerializedFileReader};

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use q_compress::{auto_compress, auto_decompress, DEFAULT_COMPRESSION_LEVEL};
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

fn compare_compress_i64(c: &mut Criterion) {
	// two benchmark groups
    let mut group = c.benchmark_group("compression");

	// plot config
	let plot_config_compress = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

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

	let mut uncompressed_i64: Vec<i64> = Vec::new();

	extract_column_data_from_parquet(&std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS"), 
			"timestamp", 
			&mut uncompressed_i64);
	
	// println!("Obtained data: {:?}", uncompressed_i64);

	let orig_i64_len = uncompressed_i64.len();
	let orig_u8_len = orig_i64_len * std::mem::size_of::<i64>();

	println!("uncompressed_i64: {} items {} bytes", orig_i64_len, orig_u8_len);

	let mut uncompressed_u8 : Vec<u8> = Vec::with_capacity(orig_u8_len);
	uncompressed_u8.resize(orig_u8_len, 0u8);
	BigEndian::write_i64_into(&uncompressed_i64, &mut uncompressed_u8);

	println!("uncompressed_u8: {} items {} bytes", uncompressed_u8.len(), 
		uncompressed_u8.len() * std::mem::size_of::<u8>());
	
	let mut compressed = Vec::with_capacity(orig_u8_len);

	let mut unpacked_i64: Vec<i64> = Vec::with_capacity(orig_i64_len);
	let mut unpacked_u8: Vec<u8> = Vec::with_capacity(orig_u8_len);

	group.throughput(Throughput::Elements(orig_u8_len as u64));
    group.bench_function(BenchmarkId::new("pack", "Q"), |b| {
		let orig = &uncompressed_i64;
        b.iter(|| {
            black_box(&mut compressed).clear();
			compressed = auto_compress(black_box(orig), DEFAULT_COMPRESSION_LEVEL)
        })
    });
    println!("Q: {} {} {} {} bytes compression_ratio {:.2}", 
		uncompressed_i64.len(), uncompressed_i64.len(), 
		uncompressed_i64.len()*std::mem::size_of::<i64>(), compressed.len(),
		orig_u8_len as f32 / compressed.len() as f32
	);
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "Q"), |b| {
        b.iter(|| {
            black_box(&mut unpacked_i64).clear();
			unpacked_i64 = auto_decompress::<i64>(black_box(&compressed)).expect("failed to decompress")
        })
    });

	// various LZ4 implementtions
	group.throughput(Throughput::Elements(orig_u8_len as u64));
    group.bench_function(BenchmarkId::new("pack", "lz4"), |b| {
		let orig = &uncompressed_u8;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_compression::compress::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz4-compression: {} {} bytes compression_ratio {:.2}", uncompressed_u8.len(), compressed.len(), orig_u8_len as f32 / compressed.len() as f32);
	group.throughput(Throughput::Elements(compressed.len() as u64));
    group.bench_function(BenchmarkId::new("unpack", "lz4"), |b| {
        b.iter(|| {
            black_box(&mut unpacked_u8).clear();
            lz4_compression::decompress::decompress_into(black_box(&compressed), black_box(&mut unpacked_u8))
        })
    });

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
