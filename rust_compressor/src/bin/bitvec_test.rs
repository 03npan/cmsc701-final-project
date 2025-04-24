use std::env;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use rkyv::Archived;
// use rkyv::{rancor::Error, Archive, Deserialize, Serialize};
use rkyv::rancor::Error;
use itertools::Itertools;
use minimum_redundancy::{Coding, DecodingResult, BitsPerFragment}; //, Code};
use huffman_compress2::{CodeBuilder, Tree}; //, Book};
use serde::{Serialize, Deserialize};
use bit_vec::BitVec;
use bitfield_rle;

// note that we will be reading row-major order while the original
// matrix.mtx is column-major order
// NOTE: we are using dense matrix but should be using sparse as input
// new idea: use rank & select enabled bitvectors to store column indices
    // can do by row (needs per row store of values) or for entire 1D version of matrix
// will need run length encoding and/or Huffman encoding of bytes to compress well
// bitvec
    // https://crates.io/crates/bitm
    // https://crates.io/crates/rsdict
    // https://crates.io/crates/vers-vecs - has serde
    // https://crates.io/crates/sux
    // https://crates.io/crates/succinct
// huffman coding:
    // https://crates.io/crates/minimum_redundancy
        // 1.83 mb with rle then huffman on split by row
        // 1.83 mb with only rle on split by row
        // 1.46 mb with only rle on 1D matrix
        // 1.44 mb with rle then huffman on 1D matrix
    // https://crates.io/crates/huffman-compress2
        // 1.69 mb with rle then huffman on split by row
        // 1.62 mb with only rle on split by row
        // 1.46 mb with only rle on 1D matrix
        // 1.45 mb with rle then huffman on 1D matrix
// rle:
    // https://crates.io/crates/bitfield-rle
// other compression:
    // https://crates.io/crates/arcode
    // https://crates.io/crates/lz4_flex
    // https://crates.io/crates/rust-lzma
    // https://docs.rs/xz2/0.1.7/xz2/

// #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[derive(Serialize, Deserialize, Debug)]
struct CompressedMatrix {
    num_features: u32,
    num_barcodes: u32,
    num_entries: u32,
    // matrix: Vec<Vec<u8>>, // rle, split by row
    matrix: Vec<u8>, // rle, 1D array
    // matrix: BitVec, // rle + huffman, 1D array
    // matrix: Vec<BitVec>, // rle + huffman, split by row
    // values: Vec<BitVec>, // split by row
    values: BitVec, // 1D array
    // tree: Tree<u32>, // if using combined tree
    // --------- for huffman-compress2 ---------
    // matrix_tree: Tree<u8>, // if using huffman on matrix
    // values_tree: Tree<u32>,
    // --------- for minimum_redundancy ---------
    // matrix_coding_values: Box<[u8]>, // u8 instead of u32 in this approach
    // matrix_coding_node_count: Box<[u32]>, // matrix_coding if using huffman on matrix
    value_coding_values: Box<[u32]>,
    value_coding_node_count: Box<[u32]>,
}

fn read_matrix_csv(csv: String) -> Vec<Vec<u32>> {
    let mut matrix:Vec<Vec<u32>> = Vec::new();

    let f = File::open(csv);
    let input = match f {
        Ok(f) => f,
        Err(error) => panic!("Can't open file: {}", error),
    };
    let reader = BufReader::new(input);

    for line in reader.lines().map(|l| l.unwrap()) {
        let mut row: Vec<u32> = Vec::new();
        let collection: Vec<&str> = line.split(",").collect();
        // let row_elems = &collection[3..];
        // eprintln!("{:?}", row_elems);
        for elem in collection {
            row.push(elem.parse::<u32>().unwrap());
        }
        matrix.push(row);
    }

    return matrix;
}

fn main() {
    let matrix:Vec<Vec<u32>>;
    let arg1 = env::args().nth(1);
    if arg1.is_some() {
        let arg2 = env::args().nth(2);
        if arg2.is_some() {
            matrix = read_matrix_csv("../mex_matrix_no_headers.csv".to_string());
        
            println!("# features: {}", matrix.len());
            println!("# barcodes:  {}", matrix[0].len());
            
            let bytes = rkyv::to_bytes::<Error>(&matrix).unwrap();
            let f = File::create("../sparse_mtx");
            let mut outfile = match f {
                Ok(f) => f,
                Err(error) => panic!("Can't create file: {}", error),
            };
            // write bytes to disk
            let res = outfile.write_all(&bytes);
            let _ = match res {
                Ok(res) => res,
                Err(error) => panic!("Can't write to file: {}", error),
            };
        }
        else {
            let f = fs::read("../sparse_mtx");
            let binary = match f {
                Ok(f) => f,
                Err(error) => panic!("Can't open file: {}", error),
            };
            let archived = rkyv::access::<Archived<Vec<Vec<u32>>>, Error>(&binary[..]).unwrap();
        
            matrix =  rkyv::deserialize::<Vec<Vec<u32>>, Error>(archived).unwrap();
        
            println!("# features: {}", matrix.len());
            println!("# barcodes:  {}", matrix[0].len());
        }

        // now we have a sparse NxM matrix loaded - begin compressing

        let mut num_entries: u32 = 0;
        let mut max_value = 0;
        // let mut max_value_delta = 0;
        // let mut rows: Vec<Vec<u32>> = Vec::new();
        let mut cols: Vec<u32> = Vec::new();
        // let mut values: Vec<Vec<u32>> = Vec::new();
        let mut values: Vec<u32> = Vec::new();
        // let mut last_value = 0;

        let mut max_gap = 0;
        let mut curr_gap = 0;

        for (_row_num, row) in matrix.iter().enumerate() {
            // let mut col_indices = Vec::new();
            // let mut values_in_row = Vec::new();
            // let mut last_col = 0;
            for (col_num, value) in row.iter().enumerate() {
                if *value != 0 {
                    cols.push((col_num + (_row_num * matrix[0].len())) as u32);
                    // col_indices.push(col_num as u32);
                    // last_col = col_num;
                    // last_col = col_num + (_row_num * compressed_matrix.num_barcodes as usize); // for 2-array alg
                    // values_in_row.push(*value);
                    values.push(*value);
                    if *value > max_value {
                        max_value = *value;
                    }
                    // let value_delta = *value as i32 - last_value as i32;
                    // values.push(value_delta);
                    // if value_delta > max_value_delta {
                    //     max_value_delta = value_delta;
                    // }
                    // last_value = *value;
                    num_entries += 1;
                    if curr_gap > max_gap {
                        max_gap = curr_gap;
                        curr_gap = 0;
                    }
                }
                else {
                    curr_gap += 1;
                }
            }
            // rows.push(col_indices);
            // values.push(values_in_row);
        }
        println!("max_value: {}", max_value);
        // println!("len col: {}", cols.len());
        // println!("len val: {}", values.len());
        println!("max_gap: {}", max_gap);

        // let value_counts = values.clone().into_iter().flatten().collect_vec().iter().copied().counts();
        let value_counts = values.iter().copied().counts();
        println!("value_vec_counts: {:?}", value_counts.values().sorted().len());

        let bits_per_value = (max_value as f64).log2().ceil() as u8;
        println!("bits per value: {}", bits_per_value);

        // --------- coding for huffman-compress2 ---------
        // let (values_book, values_tree) = CodeBuilder::from_iter(value_counts).finish();

        // --------- split by row ---------
        // let mut values_bitvectors: Vec<BitVec> = Vec::new();
        // for value in &values {
        //     let mut value_buffer = BitVec::new();
        //     for v in value {
        //         let _ = values_book.encode(&mut value_buffer, v);
        //     }
        //     values_bitvectors.push(value_buffer);
        // }

        // --------- 1D matrix ---------
        // let mut value_buffer = BitVec::new();
        // for v in &values {
        //     let _ = values_book.encode(&mut value_buffer, v);
        // }

        // --------- split by row ---------
        // let mut matrix_rle_bitvecs: Vec<Vec<u8>> = Vec::new();
        // for row in rows {
        //     let mut bitvec = BitVec::from_elem(matrix[0].len(), false);
        //     for c in row {
        //         bitvec.set(c as usize, true);
        //     }
        //     let rle_bitvec = bitfield_rle::encode(bitvec.to_bytes());
        //     // let bitvec_bytes = bitvec.to_bytes();
        //     matrix_rle_bitvecs.push(rle_bitvec);
        // }

        // --------- 1D matrix ---------
        // let mut bitvec = BitVec::from_elem(matrix.len() * matrix[0].len(), false);
        // for c in cols {
        //     bitvec.set(c as usize, true);
        // }
        // let rle_cols = bitfield_rle::encode(bitvec.to_bytes());

        // --------- huffman-compress2 on rle matrix ---------
        // let rle_counts = matrix_rle_bitvecs.clone().into_iter().flatten().collect_vec().iter().copied().counts();
        // let rle_counts = rle_cols.iter().copied().counts();
        // let (matrix_book, matrix_tree) = CodeBuilder::from_iter(rle_counts).finish();

        // --------- split by row ---------
        // let mut matrix_bitvectors: Vec<BitVec> = Vec::new();
        // for row in matrix_rle_bitvecs {
        //     let mut matrix_buffer = BitVec::new();
        //     for c in row {
        //         let _ = matrix_book.encode(&mut matrix_buffer, &c);
        //     }
        //     matrix_bitvectors.push(matrix_buffer);
        // }

        // --------- 1D matrix ---------
        // let mut matrix_buffer = BitVec::new();
        // for byte in rle_cols {
        //     let _ = matrix_book.encode(&mut matrix_buffer, &byte);
        // }
        
        // --------- coding for minimum_redundancy ---------
        let value_coding = Coding::from_frequencies(BitsPerFragment(1), value_counts);
        let values_book = value_coding.reversed_codes_for_values();

        // --------- split by row ---------
        // let mut values_bitvectors: Vec<BitVec> = Vec::new();
        // for value in &values {
        //     let mut value_buffer = BitVec::new();
        //     for v in value {
        //         let code = values_book.get(v).unwrap();
        //         let mut bits = code.content;
        //         for _ in 0..code.len {
        //             value_buffer.push((bits & 1) == 1);
        //             bits >>= 1;
        //         }
        //     }
        //     values_bitvectors.push(value_buffer);
        // }

        // --------- 1D matrix ---------
        let mut value_buffer = BitVec::new();
        for v in &values {
            let code = values_book.get(v).unwrap();
            let mut bits = code.content;
            for _ in 0..code.len {
                value_buffer.push((bits & 1) == 1);
                bits >>= 1;
            }
        }

        // --------- split by row ---------
        // let mut matrix_rle_bitvecs: Vec<Vec<u8>> = Vec::new();
        // for row in rows {
        //     let mut bitvec = BitVec::from_elem(matrix[0].len(), false);
        //     for c in row {
        //         bitvec.set(c as usize, true);
        //     }
        //     let bitvec_bytes = bitvec.to_bytes();
        //     matrix_rle_bitvecs.push(bitvec_bytes);
        // }
        
        // --------- 1D matrix ---------
        let mut bitvec = BitVec::from_elem(matrix.len() * matrix[0].len(), false);
        for c in cols {
            bitvec.set(c as usize, true);
        }
        let rle_cols = bitfield_rle::encode(bitvec.to_bytes());

        // --------- minimum_redundancy on rle matrix ---------
        // let rle_counts = matrix_rle_bitvecs.clone().into_iter().flatten().collect_vec().iter().copied().counts();
        // // let rle_counts = rle_cols.iter().copied().counts();
        // let matrix_coding = Coding::from_frequencies(BitsPerFragment(1), rle_counts);
        // let matrix_book = matrix_coding.reversed_codes_for_values();
        
        // --------- split by row ---------
        // let mut matrix_bitvectors: Vec<BitVec> = Vec::new();
        // for row in matrix_rle_bitvecs {
        //     let mut matrix_buffer = BitVec::new();
        //     for c in row {
        //         let code = matrix_book.get(&c).unwrap();
        //         let mut bits = code.content;
        //         for _ in 0..code.len {
        //             matrix_buffer.push((bits & 1) == 1);
        //             bits >>= 1;
        //         }
        //     }
        //     matrix_bitvectors.push(matrix_buffer);
        // }

        // --------- 1D matrix ---------
        // let mut matrix_buffer = BitVec::new();
        // for byte in rle_cols {
        //     let code = matrix_book.get(&byte).unwrap();
        //     let mut bits = code.content;
        //     for _ in 0..code.len {
        //         matrix_buffer.push((bits & 1) == 1);
        //         bits >>= 1;
        //     }
        // }

        let compressed_matrix = CompressedMatrix {
            num_features: matrix.len() as u32,
            num_barcodes: matrix[0].len() as u32,
            num_entries: num_entries,
            // matrix: matrix_rle_bitvecs, // rle, split by row
            matrix: rle_cols, // rle only, 1D array
            // matrix: matrix_buffer, // rle + huffman, 1D array
            // matrix: matrix_bitvectors, // rle + huffman, split by row
            // values: values_bitvectors, // rle, split by row
            values: value_buffer, // rle, 1D array
            // tree: tree, // for combined case
            // --------- for huffman-compress2 ---------
            // matrix_tree: matrix_tree,
            // values_tree: values_tree,
            // --------- for minimum_redundancy ---------
            // matrix_coding_values: matrix_coding.values,
            // matrix_coding_node_count: matrix_coding.internal_nodes_count,
            value_coding_values: value_coding.values,
            value_coding_node_count: value_coding.internal_nodes_count,
        };

        let f = File::create("../compressed_matrix2.bin");
        let mut outfile = match f {
            Ok(f) => f,
            Err(error) => panic!("Can't create file: {}", error),
        };
        let res = bincode::serde::encode_into_std_write(&compressed_matrix, &mut outfile, bincode::config::standard());
        let _ = match res {
            Ok(res) => res,
            Err(error) => panic!("Can't write to file: {}", error),
        };

        let f = File::open("../compressed_matrix2.bin");
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
        assert_eq!(compressed_matrix.matrix, compressed_matrix2.matrix);
        assert_eq!(compressed_matrix.values, compressed_matrix2.values);
        
        // --------- test for huffman-compress2 ---------
        // // let rle_row: Vec<u8> = compressed_matrix2.matrix_tree.decoder(&compressed_matrix2.matrix, 4 * compressed_matrix.num_barcodes as usize).collect();
        // // let decoded_values: Vec<u32> = compressed_matrix2.values_tree.decoder(&compressed_matrix2.values[3], compressed_matrix.num_barcodes as usize).collect();
        // let decoded_values: Vec<u32> = compressed_matrix2.values_tree.decoder(&compressed_matrix2.values, compressed_matrix.num_entries as usize).collect();
        // // let decoded_row = bitfield_rle::decode(&compressed_matrix2.matrix[3]).unwrap(); // decode for split by row rle
        // // let mut decoded_row = bitfield_rle::decode(rle_row).unwrap(); // decode for rle + huffman of 1D array
        // let mut decoded_row = bitfield_rle::decode(&compressed_matrix2.matrix).unwrap();
        // let bits = BitVec::from_bytes(&decoded_row); // this is how to turn back into rank/select bitvec
        // // for byte in decoded_row
        // for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
        //     print!("{:08b}", byte); // should return three 1s
        // }
        // assert_eq!(values, decoded_values); // decoded_values should be either num_barcodes or num_entries if doing split by row or 1D array
        
        // --------- test for minimum_redundancy ---------
        // let row_coding2 = Coding {
        //     values: compressed_matrix2.matrix_coding_values,
        //     internal_nodes_count: compressed_matrix2.matrix_coding_node_count,
        //     degree: BitsPerFragment(1)
        // };
        // let mut bytes = Vec::new();
        // let mut row_decoder = row_coding2.decoder();
        // // let mut fragments = compressed_matrix2.matrix[3].iter().peekable();
        // // while fragments.peek().is_some() {
        // //     if let DecodingResult::Value(v) = row_decoder.decode_next(&mut fragments) {
        // //         bytes.push(*v);
        // //     }
        // // }
        // // let decoded_row = bitfield_rle::decode(bytes).unwrap();
        // // for byte in decoded_row {
        // //     print!("{:08b}", byte); // should return three 1s
        // // }
        // let mut fragments: std::iter::Peekable<std::slice::Iter<'_, BitVec>> = compressed_matrix2.matrix.iter().peekable();
        // let mut count = 0;
        // while fragments.peek().is_some() {
        //     if let DecodingResult::Value(v) = row_decoder.decode_next(&mut fragments) {
        //         bytes.push(*v);
        //         count += 1;
        //     }
        //     if count > (4 * compressed_matrix2.num_barcodes) / 8 {
        //         break;
        //     }
        // }
        // let mut decoded_row = bitfield_rle::decode(bytes).unwrap();
        // for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
        //     print!("{:08b}", byte); // should return three 1s
        // }

        // --------- rle only ---------
        let mut decoded_row = bitfield_rle::decode(&compressed_matrix2.matrix).unwrap();
        let bits = BitVec::from_bytes(&decoded_row); // this is how to turn back into rank/select bitvec
        // for byte in decoded_row
        for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
            print!("{:08b}", byte); // should return three 1s
        }
    }
    else {
        let f = File::open("../compressed_matrix.bin");
        let mut input = match f {
            Ok(f) => f,
            Err(error) => panic!("Can't open file: {}", error),
        };
        let res: Result<CompressedMatrix, bincode::error::DecodeError> = bincode::serde::decode_from_std_read(&mut input, bincode::config::standard());
        let compressed_matrix = match res {
            Ok(res) => res,
            Err(error) => panic!("Can't decode file: {}", error),
        };
        // println!("{:?}", compressed_matrix);
        // let decoded: Vec<u32> = compressed_matrix.matrix_tree.decoder(&compressed_matrix.row_vec, compressed_matrix.num_features as usize).collect();
        // these tests are specific to our file
        // assert_eq!(0, decoded[0]);
        // assert_eq!(0, decoded[1]);
        // assert_eq!(0, decoded[2]);
        // assert_eq!(3, decoded[3]);
    }
}
