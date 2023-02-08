// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Binary file to read data from a Parquet file.
//!
//! The binary can also be built from the source code and run as follows:
//! ```
//! cargo run --features=cli --bin parquet-column-export XYZ.parquet column_name output_file
//! ```
//!


extern crate parquet;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Row;
use parquet::schema::types::Type;
use parquet::basic::Type as PhysicalType;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::env;
use std::process;
use std::sync::Arc;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about("Binary file to extract a column data from a Parquet file"), long_about = None)]
struct Arguments {
    
    #[clap(subcommand)]
    cmd: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    /// show schema
    Schema {
        #[clap(short, long, help("Path to a parquet file, or - for stdin"))]
        input: String,
    },
    /// list all the projects
    Export {
        #[clap(short, long, help("Path to a parquet file, or - for stdin"))]
        input: String,
        #[clap(short, long, help("output file name"))]
        // output file name
        output: String,
        #[clap(short, long, multiple_values = true, value_delimiter = ',', help("a list of selected columns delimited by comma"))]
        // paths to exclude when searching
        columns: Vec<String>,
    },
}

fn print_schema(
		fields:&[Arc<parquet::schema::types::Type>]
	){
	
	for (pos, column) in fields.iter().enumerate() {
		let name = column.name();		       
		let p_type = column.get_physical_type();
		let output_rust_type = match p_type {					
			PhysicalType::FIXED_LEN_BYTE_ARRAY=>"String",
			PhysicalType::BYTE_ARRAY=> "String",
			PhysicalType::INT64=>"i64",
			PhysicalType::INT32=> "i32",
			PhysicalType::FLOAT => "f32",
			PhysicalType::DOUBLE=> "f64",
			_ =>panic!(
				"Cannot convert  this parquet file, unhandled data type for column {}", 
				name),									
		};
		println!("{} {} {}",pos, name, output_rust_type);			
	} // for each column	
}


fn print_data(
	reader: &SerializedFileReader<File>, 
	fields:&[Arc<parquet::schema::types::Type>], 
    output_file: &str,
	columns:Vec<String>){
	
	let delimiter = ",";	
	let requested_fields = &columns;
		
	let mut selected_fields = fields.to_vec();
	if requested_fields.len()>0{
		selected_fields.retain(|f|  
			requested_fields.contains(&String::from(f.name())));
	}			
	
	let header: String = format!("{}",
		selected_fields.iter().map(|v| v.name())
		.collect::<Vec<&str>>().join(delimiter));

    let mut row_iter = reader.get_row_iter(None).unwrap();

	println!("{}",header);

    let mut output = BufWriter::new(File::create(output_file).unwrap());

	while let Some(record) = row_iter.next() {	
        let result = record.get_column_iter()
            //.filter(|x| x.0.eq_ignore_ascii_case(args[2].as_str()))
            .filter(|x| columns.contains(x.0))
            .map(|c| c.1.to_string())
            .collect::<Vec<String>>()
            .join(delimiter);

		println!("{}",result);

        writeln!(output, "{}",result);
        
	}
	
    // some parquet files do not support schema projection

	// let schema_projection = Type::group_type_builder("schema")
	// 		.with_fields(&mut selected_fields)
	// 		.build()
	// 		.unwrap();

	// let mut row_iter = reader
	// 	.get_row_iter(Some(schema_projection)).unwrap();
	// println!("{}",header);
	// while let Some(record) = row_iter.next() {	
	// 	println!("{}",format_row(&record, &delimiter));
	// }
}

fn format_row(
		row : &parquet::record::Row, 
		delimiter: &str,
        args:&Vec<String>) -> String {  
	
	row.get_column_iter()
            //.filter(|x| x.0.eq_ignore_ascii_case(args[2].as_str()))
            .filter(|x| args[2..].contains(x.0))
		.map(|c| c.1.to_string())
		.collect::<Vec<String>>()
		.join(delimiter)
}


fn main(){

    let args = Arguments::parse();
    
    match args.cmd {
        SubCommand::Schema {  input } => {
            let file = File::open(Path::new(&input)).expect("couldn't open parquet file");
            let reader:SerializedFileReader<File> = SerializedFileReader::new(file).unwrap();
            let fields = reader.metadata()
                                            .file_metadata()
                                            .schema()
                                            .get_fields();
            print_schema(fields);
        },
        SubCommand::Export {
            input,
            output,
            columns,
        } => {
            let parquet_path = &input;	
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

            print_data(&reader, fields, &output, columns);
        }
    }
	
}	
