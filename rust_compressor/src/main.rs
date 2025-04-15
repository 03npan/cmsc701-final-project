use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use rkyv::Archived;
use rkyv::{rancor::Error, Archive, Deserialize, Serialize};
use itertools::Itertools;

// note that we will be reading row-major order while the original
// matrix.mtx is column-major order
// improvements over just storing vecs of the (row, col, val) three-tuples
    // --- ESSENTIALLY CSR BUT USING PACKED INTS ---
    // more compressed three vector format:
        // row vector stores number of values in row (index is row #)
        // col vector stores relative col #s within row
        // value vector stores delta encoded value?
            // packing negatives - fun...
        // col[i] pairs with value[i]
        // can also do the same with 1 vector format:
            // store (# values in row, (col, value), ...) for each row
            // will need to track row num when reading/decompressing
    // might be better to track largest entries in row & col vecs and
    // store # bits needed for each
        // could get even smaller if each bits used varied per row - painful to work with
    // variable length codings
        // huffman coding?
            // https://www.programminglogic.com/implementing-huffman-coding-in-c/
            // will need to store the mapping of bits to numbers
                // each vec has its own mapping to store
                // or could combine all values into one giant Huffman tree?
            // can save bits by using fewer bits for common values, provided size
            // of mapping + extra bits used for uncommon values is less than savings
            // https://docs.rs/huffman-coding/latest/huffman_coding/
            // https://crates.io/crates/minimum_redundancy
            // https://crates.io/crates/huffman-compress2
            // https://crates.io/crates/huff-tree-tap
            // https://crates.io/crates/simple_huffman
            // https://crates.io/crates/hfmn
        // fibonacci coding?
            // will need to store mapping of numbers to frequency rank
                // most common int is rank 1, second most common is 2, etc.
            // encode the rank for a num - when decoding, map rank back to number
            // very few bits for most common! fibonacci coding of 1 is just 2 bits,
            // 2 is 3 bits - larger for more uncommon numbers
            // https://docs.rs/fibonacci_codec/latest/fibonacci_codec/
            // https://en.wikipedia.org/wiki/Fibonacci_coding
        // elias universal coding?
            // https://en.wikipedia.org/wiki/Elias_coding
            // similar to fibonacci

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
struct CompressedMatrix {
    num_features: u32,
    num_barcodes: u32,
    num_entries: u32,
    bits_per_row_count: u32,
    bits_per_col: u32,
    bits_per_value: u32,
    row_vec: Vec<u32>,
    col_vec: Vec<u32>,
    values_vec: Vec<u32>,
}

fn read_matrix_csv(csv: String) -> Vec<Vec<u32>> {
    let mut matrix:Vec<Vec<u32>> = Vec::new();

    let f = File::open(csv);
    let input = match f {
        Ok(f) => f,
        Err(_error) => panic!("Can't open file"),
    };
    let reader = BufReader::new(input);

    for line in reader.lines().skip(1).map(|l| l.unwrap()) {
        let mut row: Vec<u32> = Vec::new();
        let collection: Vec<&str> = line.split(",").collect();
        let row_elems = &collection[3..];
        // eprintln!("{:?}", row_elems);
        for elem in row_elems {
            row.push(elem.parse::<u32>().unwrap());
        }
        matrix.push(row);
    }

    return matrix;
}

fn pack_bits(vec_to_pack: Vec<u32>, mask_bits: u32) -> Vec<u32> {
    // eprintln!("{}", mask_bits);
    let bit_mask = (1 << mask_bits) - 1;
    let mut packed: Vec<u32> = Vec::new();
    let mut packed_u32 = 0;

    // num bits unprocessed in current byte
    let mut bits_left_this_pack = 32;

    for v in vec_to_pack {
        // num bits to be processed for current subchunk
        let mut bits_left_this_rank = mask_bits;
        let value = v & bit_mask; // make sure we have the right bits
        while bits_left_this_rank > 0 {
            if bits_left_this_pack > bits_left_this_rank {
                packed_u32 |= value << (bits_left_this_pack - bits_left_this_rank);
                bits_left_this_pack -= bits_left_this_rank;
                bits_left_this_rank -= bits_left_this_rank;
            }
            else {
                packed_u32 |= value >> (bits_left_this_rank - bits_left_this_pack);
                bits_left_this_rank -= bits_left_this_pack;
                bits_left_this_pack -= bits_left_this_pack;
            }

            if bits_left_this_pack == 0 {
                packed.push(packed_u32);
                packed_u32 = 0;
                bits_left_this_pack = 32;
            }
        }
    }

    // leftover bits at end that don't form full u32
    if bits_left_this_pack != 32 {
        packed.push(packed_u32);
    }

    packed
}

fn main() {
    let matrix:Vec<Vec<u32>>;
    if false {
        matrix = read_matrix_csv("../mex_matrix.csv".to_string());
    
        println!("# features: {}", matrix.len());
        println!("# barcodes:  {}", matrix[0].len());
        
        let bytes = rkyv::to_bytes::<Error>(&matrix).unwrap();
        let f = File::create("../sparse_mtx");
        let mut outfile = match f {
            Ok(f) => f,
            Err(_error) => panic!("Can't create file"),
        };
        // write bytes to disk
        let res = outfile.write_all(&bytes);
        let _ = match res {
            Ok(res) => res,
            Err(_error) => panic!("Can't write to file"),
        };
    }
    else {
        let f = fs::read("../sparse_mtx");
        let binary = match f {
            Ok(f) => f,
            Err(_error) => panic!("Can't open file"),
        };
        let archived = rkyv::access::<Archived<Vec<Vec<u32>>>, Error>(&binary[..]).unwrap();
    
        matrix =  rkyv::deserialize::<Vec<Vec<u32>>, Error>(archived).unwrap();
    
        println!("# features: {}", matrix.len());
        println!("# barcodes:  {}", matrix[0].len());
    }

    // now we have a sparse NxM matrix loaded - begin compressing

    let mut compressed_matrix = CompressedMatrix {
        num_features: matrix.len() as u32,
        num_barcodes: matrix[0].len() as u32,
        num_entries: 0,
        bits_per_row_count: 0,
        bits_per_col: 0,
        bits_per_value: 0,
        // combined_vec: Vec::new(),
        // pick combined OR below three - not both
        row_vec: Vec::new(),
        col_vec: Vec::new(),
        values_vec: Vec::new(),
    };

    let mut num_entries = 0;
    let mut max_row_count = 0;
    let mut max_col_delta = 0;
    let mut max_value = 0;
    let mut row_counts: Vec<u32> = Vec::new();
    let mut cols: Vec<u32> = Vec::new();
    let mut values: Vec<u32> = Vec::new();

    // switch to storing (col, value) in hashtable with row as key?
    for (_row_num, row) in matrix.iter().enumerate() {
        let mut row_count = 0;
        let mut last_col = 0;
        for (col_num, value) in row.iter().enumerate() {
            if *value != 0 {
                // relative column nums
                let col_delta = (col_num - last_col) as u32;
                if col_delta > max_col_delta {
                    max_col_delta = col_delta;
                }
                cols.push(col_delta);
                last_col = col_num;
                values.push(*value);
                if *value > max_value {
                    max_value = *value;
                }
                num_entries += 1;
                row_count += 1;
            }
        }
        if row_count > max_row_count {
            max_row_count = row_count;
        }
        row_counts.push(row_count);
    }
    println!("max_row_count: {}", max_row_count);
    println!("max_col_delta: {}", max_col_delta);
    println!("max_value: {}", max_value); // outlier large values :(
    println!("len: {}", row_counts.len());

    let row_vec_counts = row_counts.iter().counts();
    let col_vec_counts = cols.iter().counts();
    let value_vec_counts = values.iter().counts();
    println!("row_vec_counts: {:?}", row_vec_counts.values().sorted().len());
    println!("col_vec_counts: {:?}", col_vec_counts.values().sorted().len());
    println!("value_vec_counts: {:?}", value_vec_counts.values().sorted().len());

    compressed_matrix.num_entries = num_entries;
    compressed_matrix.bits_per_row_count = (max_row_count as f64).log2().ceil() as u32;
    compressed_matrix.bits_per_col = (max_col_delta as f64).log2().ceil() as u32;
    compressed_matrix.bits_per_value = (max_value as f64).log2().ceil() as u32;
    println!("bits per row, col, value: {}, {}, {}", compressed_matrix.bits_per_row_count, compressed_matrix.bits_per_col, compressed_matrix.bits_per_value);

    compressed_matrix.row_vec = pack_bits(row_counts, compressed_matrix.bits_per_row_count);
    compressed_matrix.col_vec = pack_bits(cols, compressed_matrix.bits_per_col);
    compressed_matrix.values_vec = pack_bits(values, compressed_matrix.bits_per_value);
        
    let bytes = rkyv::to_bytes::<Error>(&compressed_matrix).unwrap();
    let f = File::create("../compressed_matrix.bin");
    let mut outfile = match f {
        Ok(f) => f,
        Err(_error) => panic!("Can't create file"),
    };
    // write bytes to disk
    let res = outfile.write_all(&bytes);
    let _ = match res {
        Ok(res) => res,
        Err(_error) => panic!("Can't write to file"),
    };
}
