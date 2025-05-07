use std::{env, fs};
use std::fs::File;
use itertools::Itertools;
use minimum_redundancy::{Coding, DecodingResult, BitsPerFragment};
// use huffman_compress2::{CodeBuilder, Tree};
use serde::{Serialize, Deserialize};
use bit_vec::BitVec;
use matrix_market_rs::MtxData;

// read mtx file natively in rust
// https://crates.io/crates/matrix-market-rs
// note that we process/store in row-major order while the original
// matrix.mtx is column-major order
// improvements over just storing vecs of the (row, col, val) three-tuples
    // --- ESSENTIALLY CSR BUT USING PACKED INTS ---
    // more compressed three vector format:
        // row vector stores number of values in row (index is row #)
        // col vector stores relative col #s within row
        // value vector stores value
        // col[i] pairs with value[i]
    // variable length codings
        // huffman coding?
            // https://www.programminglogic.com/implementing-huffman-coding-in-c/
            // will need to store the mapping of bits to numbers
                // each vec has its own mapping to store
                // or could combine all values into one giant Huffman tree?
            // can save bits by using fewer bits for common values, provided size
            // of mapping + extra bits used for uncommon values is less than savings
            // usable crates
                // https://crates.io/crates/minimum_redundancy
                    // 1.39 mb for CSR
                    // 1.41 mb for 1D array
                    // smaller on all datasets tested so far
                // https://crates.io/crates/huffman-compress2
                    // 1.43 mb for CSR
                    // 1.47 mb for 1D array

// #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[derive(Serialize, Deserialize, Debug)]
struct CompressedMatrix {
    num_features: u32,
    num_barcodes: u32,
    num_entries: u32,
    row_vec: BitVec,
    col_vec: BitVec,
    values_vec: BitVec,
    // tree: Tree<u32>, // if using combined tree
    // --------- for huffman-compress2 ---------
    // row_tree: Tree<u32>,
    // col_tree: Tree<u32>,
    // values_tree: Tree<u32>,
    // --------- for minimum_redundancy ---------
    row_coding_values: Box<[u32]>,
    row_coding_node_count: Box<[u32]>,
    col_coding_values: Box<[u32]>,
    col_coding_node_count: Box<[u32]>,
    value_coding_values: Box<[u32]>,
    value_coding_node_count: Box<[u32]>,
    // coding_values: Box<[u32]>, // these two are for combined huffman
    // coding_node_count: Box<[u32]>
}

fn read_matrix_mtx(mtx: String) -> ([usize; 2], Vec<[usize; 2]>, Vec<u32>) {
    let res = MtxData::from_file(mtx);
    let sparse = match res {
        Ok(res) => res,
        Err(error) => panic!("Can't open file: {}", error),
    };
    if let MtxData::Sparse(size, coordinates, values, _) = sparse {

        let mut combined: Vec<([usize; 2], u32)> = coordinates.into_iter().zip(values.into_iter()).collect();

        // sort into row-major order
        combined.sort_by(|a, b| a.0[0].cmp(&b.0[0]).then(a.0[1].cmp(&b.0[1])));

        let (coordinates_sorted, values_sorted): (Vec<_>, Vec<_>) = combined.into_iter().unzip();

        println!("{:?}", coordinates_sorted.len());
        println!("{:?}", values_sorted.len());

        return (size, coordinates_sorted, values_sorted);
    }
    else {
        panic!("Not sparse mtx file (maybe is dense file???)");
    }
}

fn main() {
    let arg1 = env::args().nth(1);
    let arg2 = env::args().nth(2);
    if arg1.is_some() && arg2.is_some() {
        let (size, coordinates, mtx_values) = read_matrix_mtx(arg1.unwrap());
        println!("# features: {}", size[0]);
        println!("# barcodes:  {}", size[1]);
        let num_entries = coordinates.len();

        // now we have a sparse NxM matrix loaded - begin compressing

        let mut max_row_count = 0;
        let mut max_col_delta = 0;
        let mut max_value = 0;
        // let mut max_row_count_delta = 0;
        // let mut max_col_delta = 0;
        // let mut max_value_delta = 0;
        let mut row_counts: Vec<u32> = Vec::new();
        let mut cols: Vec<u32> = Vec::new();
        let mut values: Vec<u32> = Vec::new();
        // let mut last_row_count = 0;
        // let mut last_col = 0;
        // let mut last_value = 0;

        // let mut last_col = 0; // for 2-array alg - seems pretty close to 3-array alg

        let mut row_count = 0;
        let mut last_row: i32 = -1;
        let mut last_col = 0;
        for idx in 0..num_entries {
            let row = coordinates[idx][0];
            let col = coordinates[idx][1];
            let value = mtx_values[idx];

            if row as i32 != last_row {
                if row_count != 0 {
                    if row_count > max_row_count {
                        max_row_count = row_count;
                    }
                    row_counts.push(row_count);
                    // let row_delta = row_count - last_row_count;
                    // if row_delta > max_row_count_delta {
                    //     max_row_count_delta = row_delta;
                    // }
                    // row_counts.push(row_delta);
                    // last_row_count = row_count
                }
                let num_empty_rows_between = row as i32 - last_row - 1;
                for _ in 0..num_empty_rows_between {
                    row_counts.push(0); // account for empty rows
                }
                last_row = row as i32;
                row_count = 1;
                last_col = 0;
            }
            else {
                row_count += 1;
            }

            // relative column nums - seems to be only delta encoding that has significant benefits
            // let col_delta = ((col_num + (_row_num * matrix[0].len() as usize)) - last_col) as u32;  // for 2-array alg
            // let col_delta = col_num as i32 - last_col as i32; // if not resetting per row
            let col_delta = col - last_col;
            if col_delta > max_col_delta {
                max_col_delta = col_delta;
            }
            cols.push(col_delta as u32);
            last_col = col;
            // last_col = col_num + (_row_num * matrix[0].len() as usize); // for 2-array alg
            values.push(value);
            if value > max_value {
                max_value = value;
            }
            // let value_delta = value as i32 - last_value as i32;
            // values.push(value_delta);
            // if value_delta > max_value_delta {
            //     max_value_delta = value_delta;
            // }
            // last_value = value;
        }

        println!("max_row_count: {}", max_row_count);
        println!("max_col_delta: {}", max_col_delta);
        println!("max_value: {}", max_value);
        println!("len row: {}", row_counts.len());
        // println!("len col: {}", cols.len());
        // println!("len val: {}", values.len());

        let row_vec_counts = row_counts.iter().copied().counts();
        let col_vec_counts = cols.iter().copied().counts();
        let value_vec_counts = values.iter().copied().counts();
        println!("row_vec_counts: {:?}", row_vec_counts.values().sorted().len());
        println!("col_vec_counts: {:?}", col_vec_counts.values().sorted().len());
        println!("value_vec_counts: {:?}", value_vec_counts.values().sorted().len());
        // let mut combined = Vec::new();
        // combined.append(&mut row_counts.clone());
        // combined.append(&mut cols.clone());
        // combined.append(&mut values.clone());
        // println!("len combined: {}", combined.len());
        // let combined_counts = combined.iter().copied().counts();
        // println!("combined_counts: {:?}", combined_counts.values().sorted().len());

        let bits_per_row_count = (max_row_count as f64).log2().ceil() as u8;
        let bits_per_col = (max_col_delta as f64).log2().ceil() as u8;
        let bits_per_value = (max_value as f64).log2().ceil() as u8;
        println!("bits per row, col, value: {}, {}, {}", bits_per_row_count, bits_per_col, bits_per_value);

        // --------- coding for huffman-compress2 ---------
        // let (row_book, row_tree) = CodeBuilder::from_iter(row_vec_counts).finish();
        // let (col_book, col_tree) = CodeBuilder::from_iter(col_vec_counts).finish();
        // let (values_book, values_tree) = CodeBuilder::from_iter(value_vec_counts).finish();
        // // let (book, tree) = CodeBuilder::from_iter(combined_counts).finish();

        // let mut row_buffer = BitVec::new();
        // for r in &row_counts {
        //     let _ = row_book.encode(&mut row_buffer, r);
        // }
        // let mut col_buffer = BitVec::new();
        // for c in &cols {
        //     let _ = col_book.encode(&mut col_buffer, c);
        // }
        // let mut value_buffer = BitVec::new();
        // for v in &values {
        //     let _ = values_book.encode(&mut value_buffer, v);
        // }
        
        // --------- coding for minimum_redundancy ---------
        let row_coding = Coding::from_frequencies(BitsPerFragment(1), row_vec_counts);
        let col_coding = Coding::from_frequencies(BitsPerFragment(1), col_vec_counts);
        let value_coding = Coding::from_frequencies(BitsPerFragment(1), value_vec_counts);
        // let coding = Coding::from_frequencies(BitsPerFragment(1), combined_counts);
        
        let row_book = row_coding.reversed_codes_for_values();
        let col_book = col_coding.reversed_codes_for_values();
        let values_book = value_coding.reversed_codes_for_values();
        // let book = coding.reversed_codes_for_values();
        
        let mut row_buffer = BitVec::new();
        for r in &row_counts {
            let code = row_book.get(r).unwrap();
            let mut bits = code.content;
            for _ in 0..code.len {
                row_buffer.push((bits & 1) == 1);
                bits >>= 1;
            }
        }
        let mut col_buffer = BitVec::new();
        for c in &cols {
            let code = col_book.get(c).unwrap();
            let mut bits = code.content;
            for _ in 0..code.len {
                col_buffer.push((bits & 1) == 1);
                bits >>= 1;
            }
        }
        let mut value_buffer = BitVec::new();
        for v in &values {
            let code = values_book.get(v).unwrap();
            let mut bits = code.content;
            for _ in 0..code.len {
                value_buffer.push((bits & 1) == 1);
                bits >>= 1;
            }
        }

        let compressed_matrix = CompressedMatrix {
            num_features: size[0] as u32,
            num_barcodes: size[1] as u32,
            num_entries: num_entries as u32,
            row_vec: row_buffer,
            col_vec: col_buffer,
            values_vec: value_buffer,
            // tree: tree, // for combined case
            // --------- for huffman-compress2 ---------
            // row_tree: row_tree,
            // col_tree: col_tree,
            // values_tree: values_tree,
            // --------- for minimum_redundancy ---------
            row_coding_values: row_coding.values,
            row_coding_node_count: row_coding.internal_nodes_count,
            col_coding_values: col_coding.values,
            col_coding_node_count: col_coding.internal_nodes_count,
            value_coding_values: value_coding.values,
            value_coding_node_count: value_coding.internal_nodes_count,
            // coding_values: coding.values,
            // coding_node_count: coding.internal_nodes_count
        };

        let output_dir = arg2.unwrap();
        let _ = fs::create_dir(&output_dir);
        let f = File::create(format!("{output_dir}/compressed_matrix.bin"));
        let mut outfile = match f {
            Ok(f) => f,
            Err(error) => panic!("Can't create file: {}", error),
        };
        let res = bincode::serde::encode_into_std_write(&compressed_matrix, &mut outfile, bincode::config::standard());
        let _ = match res {
            Ok(res) => res,
            Err(error) => panic!("Can't write to file: {}", error),
        };

        let f = File::open(format!("{output_dir}/compressed_matrix.bin"));
        let mut input = match f {
            Ok(f) => f,
            Err(error) => panic!("Can't open file: {}", error),
        };
        let res: Result<CompressedMatrix, bincode::error::DecodeError> = bincode::serde::decode_from_std_read(&mut input, bincode::config::standard());
        let compressed_matrix2 = match res {
            Ok(res) => res,
            Err(error) => panic!("Can't decode file: {}", error),
        };
        // println!("{:?}", compressed_matrix);
        assert_eq!(compressed_matrix.num_features, compressed_matrix2.num_features);
        assert_eq!(compressed_matrix.num_barcodes, compressed_matrix2.num_barcodes);
        assert_eq!(compressed_matrix.num_entries, compressed_matrix2.num_entries);
        assert_eq!(compressed_matrix.row_vec, compressed_matrix2.row_vec);
        assert_eq!(compressed_matrix.col_vec, compressed_matrix2.col_vec);
        assert_eq!(compressed_matrix.values_vec, compressed_matrix2.values_vec);

        // --------- test for huffman-compress2 huffman coding ---------
        // let decoded_row: Vec<u32> = compressed_matrix2.row_tree.decoder(&compressed_matrix2.row_vec, compressed_matrix.num_features as usize).collect();
        // let decoded_col: Vec<u32> = compressed_matrix2.col_tree.decoder(&compressed_matrix2.col_vec, compressed_matrix.num_entries as usize).collect();
        // let decoded_values: Vec<u32> = compressed_matrix2.values_tree.decoder(&compressed_matrix2.values_vec, compressed_matrix.num_entries as usize).collect();
        // assert_eq!(row_counts, decoded_row);
        // assert_eq!(cols, decoded_col);
        // assert_eq!(values, decoded_values);

        // --------- test for minimum_redundancy huffman coding ---------
        let row_coding2 = Coding {
            values: compressed_matrix2.row_coding_values,
            internal_nodes_count: compressed_matrix2.row_coding_node_count,
            // values: compressed_matrix2.coding_values,
            // internal_nodes_count: compressed_matrix2.coding_node_count,
            degree: BitsPerFragment(1)
        };
        let mut row_decoder = row_coding2.decoder();
        let mut num_decoded = 0;
        let mut fragments = compressed_matrix2.row_vec.iter();// as bit_vec::Iter<u32>;
        while num_decoded < 4 {
            if let DecodingResult::Value(v) = row_decoder.decode_next(&mut fragments) {
                // should return 0,0,0,3 for test1
                // should return 0,0,0,125 for test2
                // should return 0,1,0,111 for test3
                // should return 1,0,0,54 for test4
                // should return 32525,264107,63667,28605 for test5
                println!("{}", v);
                num_decoded += 1;
            }
        }
    }
    else {
        eprintln!("Usage: rust_compressor <input_mtx> <output_dir>");
    }
}
