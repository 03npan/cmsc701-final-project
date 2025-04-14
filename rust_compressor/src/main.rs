use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use rkyv::Archived;
use rkyv::{rancor::Error, Archive, Deserialize, Serialize};

// first store [# features], [# barcodes], [# entries in this file]
// these should probably be u32s
// next, for each non-zero entry store the following:
// [row #] [col #] [value]
// these should be packed ints
// lg([# features]) and lg([# barcodes]) gives us size of ints used
// to store those entries - not sure about int size of [value]
// easiest to just store [bits per entry] up front as well
// note that we will be reading row-major order while the original
// matrix.mtx is column-major order
// two ways to do this:
    // 1. store vector of (row, col, value) three-tuples
    // 2. store 3 vectors: one for row, col, and value
// possible improvements
    // don't duplicate storing row values (if reading row-major order)
        // big savings here if all non-zero row values are stored consecutively
    // store relative col # instead of absolute
    // values should be delta encoded as well
    // more compressed three vector format:
        // row vector stores number of values in row (index is row #)
        // col vector stores relative col #s within row
        // value vector stores delta encoded value
            // packing negatives - fun...
        // col[i] pairs with value[i]
        // can also do the same with 1 vector format:
            // store (# values in row, (col, value), ...) for each row
            // will need to track row num when reading/decompressing
    // might be better to track largest entries in row & col vecs and
    // store # bits needed for each
        // could get even smaller if each bits used varied per row - painful to work with
    // huffman encoding? https://www.programminglogic.com/implementing-huffman-coding-in-c/
        // probably requires the three vector solution

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
struct CompressedMatrix {
    num_features: u32,
    num_barcodes: u32,
    num_entries: u32,
    bits_per_value: u32,
    combined_vec: Vec<u32>,
    // pick combined OR below three - not both
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
        bits_per_value: 0,
        combined_vec: Vec::new(),
        // pick combined OR below three - not both
        row_vec: Vec::new(),
        col_vec: Vec::new(),
        values_vec: Vec::new(),
    };

    let mut num_entries = 0;
    let mut max_value = 0;

    for (row_num, row) in matrix.iter().enumerate() {
        for (col_num, value) in row.iter().enumerate() {
            if *value != 0 {
                compressed_matrix.combined_vec.push(row_num as u32);
                compressed_matrix.combined_vec.push(col_num as u32);
                compressed_matrix.combined_vec.push(*value);
            }
        }
    }

    // println!("{:?}", compressed_matrix.combined_vec);
        
    let bytes = rkyv::to_bytes::<Error>(&compressed_matrix).unwrap();
    let f = File::create("../compressed_matrix");
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
