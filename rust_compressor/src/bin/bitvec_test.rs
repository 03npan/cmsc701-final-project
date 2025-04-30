use std::{env, fs};
use std::fs::File;
use itertools::Itertools;
use minimum_redundancy::{Coding, BitsPerFragment};
// use huffman_compress2::{CodeBuilder, Tree};
use serde::{Serialize, Deserialize};
use bit_vec::BitVec;
use bitfield_rle;
use matrix_market_rs::MtxData;

// read mtx file natively in rust
// https://crates.io/crates/matrix-market-rs
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
    matrix: Vec<u8>, // rle, 1D array
    // matrix: BitVec, // rle + huffman, 1D array
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

        let mut max_value = 0;
        // let mut max_value_delta = 0;
        // let mut rows: Vec<Vec<u32>> = Vec::new();
        let mut cols: Vec<u32> = Vec::new();
        // let mut values: Vec<Vec<u32>> = Vec::new();
        let mut values: Vec<u32> = Vec::new();
        // let mut last_value = 0;

        for idx in 0..num_entries {
            let row = coordinates[idx][0];
            let col = coordinates[idx][1];
            let value = mtx_values[idx];

            cols.push((col + (row * size[1])) as u32);
            // col_indices.push(col_num as u32);
            // last_col = col_num;
            // last_col = col_num + (_row_num * compressed_matrix.num_barcodes as usize); // for 2-array alg
            // values_in_row.push(*value);
            values.push(value);
            if value > max_value {
                max_value = value;
            }
            // let value_delta = *value as i32 - last_value as i32;
            // values.push(value_delta);
            // if value_delta > max_value_delta {
            //     max_value_delta = value_delta;
            // }
            // last_value = *value;
        }

        println!("max_value: {}", max_value);
        // println!("len col: {}", cols.len());
        // println!("len val: {}", values.len());

        // let value_counts = values.clone().into_iter().flatten().collect_vec().iter().copied().counts();
        let value_counts = values.iter().copied().counts();
        println!("value_vec_counts: {:?}", value_counts.values().sorted().len());

        let bits_per_value = (max_value as f64).log2().ceil() as u8;
        println!("bits per value: {}", bits_per_value);

        // --------- coding for huffman-compress2 ---------
        // let (values_book, values_tree) = CodeBuilder::from_iter(value_counts).finish();

        // let mut value_buffer = BitVec::new();
        // for v in &values {
        //     let _ = values_book.encode(&mut value_buffer, v);
        // }
        
        // let mut bitvec = BitVec::from_elem(size[0] * size[1], false);
        // for c in cols {
        //     bitvec.set(c as usize, true);
        // }
        // let rle_cols = bitfield_rle::encode(bitvec.to_bytes());

        // --------- huffman-compress2 on rle matrix ---------
        // let rle_counts = rle_cols.iter().copied().counts();
        // let (matrix_book, matrix_tree) = CodeBuilder::from_iter(rle_counts).finish();

        // let mut matrix_buffer = BitVec::new();
        // for byte in rle_cols {
        //     let _ = matrix_book.encode(&mut matrix_buffer, &byte);
        // }
        
        // --------- coding for minimum_redundancy ---------
        let value_coding = Coding::from_frequencies(BitsPerFragment(1), value_counts);
        let values_book = value_coding.reversed_codes_for_values();

        let mut value_buffer = BitVec::new();
        for v in &values {
            let code = values_book.get(v).unwrap();
            let mut bits = code.content;
            for _ in 0..code.len {
                value_buffer.push((bits & 1) == 1);
                bits >>= 1;
            }
        }
        
        let mut bitvec = BitVec::from_elem(size[0] * size[1], false);
        for c in cols {
            bitvec.set(c as usize, true);
        }
        let rle_cols = bitfield_rle::encode(bitvec.to_bytes());

        // --------- minimum_redundancy on rle matrix ---------
        // let rle_counts = rle_cols.iter().copied().counts();
        // let matrix_coding = Coding::from_frequencies(BitsPerFragment(1), rle_counts);
        // let matrix_book = matrix_coding.reversed_codes_for_values();

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
            num_features: size[0] as u32,
            num_barcodes: size[1] as u32,
            num_entries: num_entries as u32,
            matrix: rle_cols, // rle only, 1D array
            // matrix: matrix_buffer, // rle + huffman, 1D array
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

        let output_dir = arg2.unwrap();
        let _ = fs::create_dir(&output_dir);
        let f = File::create(format!("{output_dir}/compressed_matrix_bv.bin"));
        let mut outfile = match f {
            Ok(f) => f,
            Err(error) => panic!("Can't create file: {}", error),
        };
        let res = bincode::serde::encode_into_std_write(&compressed_matrix, &mut outfile, bincode::config::standard());
        let _ = match res {
            Ok(res) => res,
            Err(error) => panic!("Can't write to file: {}", error),
        };

        let f = File::open(format!("{output_dir}/compressed_matrix_bv.bin"));
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
        // let decoded_values: Vec<u32> = compressed_matrix2.values_tree.decoder(&compressed_matrix2.values, compressed_matrix.num_entries as usize).collect();
        // // let mut decoded_row = bitfield_rle::decode(rle_row).unwrap(); // decode for rle + huffman of 1D array
        // let mut decoded_row = bitfield_rle::decode(&compressed_matrix2.matrix).unwrap();
        // let bits = BitVec::from_bytes(&decoded_row); // this is how to turn back into rank/select bitvec
        // // for byte in decoded_row
        // let mut ones = 0;
        // for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
        //     ones += byte.count_ones();
        //     // print!("{:08b}", byte);
        // }
        // // should return 3 for test1
        // // should return 125 for test2
        // // should return 111 for test3
        // // should return 54  for test4
        // // should return 28605 for test5
        // println!("{}", ones);
        // assert_eq!(values, decoded_values); // decoded_values should be either num_barcodes or num_entries if doing split by row or 1D array
        
        // --------- test for minimum_redundancy ---------
        // let row_coding2 = Coding {
        //     values: compressed_matrix2.matrix_coding_values,
        //     internal_nodes_count: compressed_matrix2.matrix_coding_node_count,
        //     degree: BitsPerFragment(1)
        // };
        // let mut bytes = Vec::new();
        // let mut row_decoder = row_coding2.decoder();
        // let mut fragments = compressed_matrix2.matrix.iter().peekable();
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
        // let mut ones = 0;
        // for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
        //     ones += byte.count_ones();
        //     // print!("{:08b}", byte);
        // }
        // // should return 3 for test1
        // // should return 125 for test2
        // // should return 111 for test3
        // // should return 54  for test4
        // // should return 28605 for test5
        // println!("{}", ones);

        // --------- rle only ---------
        let mut decoded_row = bitfield_rle::decode(&compressed_matrix2.matrix).unwrap();
        let _bits = BitVec::from_bytes(&decoded_row); // this is how to turn back into rank/select bitvec
        // for byte in decoded_row
        let mut ones = 0;
        for byte in &mut decoded_row[3*(compressed_matrix2.num_barcodes as usize/8)..4*(compressed_matrix2.num_barcodes as usize/8)] {
            ones += byte.count_ones();
            // print!("{:08b}", byte);
        }
        // should return 3 for test1
        // should return 125 for test2
        // should return 111 for test3
        // should return 54  for test4
        // should return 28605 for test5
        println!("{}", ones);
    }
    else {
        eprintln!("Usage: rust_compressor <input_mtx> <output>");
    }
}
