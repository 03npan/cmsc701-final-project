use std::env;
use matrix_market_rs::{MtxData, MtxError};

// read mtx file natively in rust
// https://crates.io/crates/matrix-market-rs

fn main() -> Result<(), MtxError> {
    let arg1 = env::args().nth(1);
    if arg1.is_some() {
        let sparse: MtxData<u32> = MtxData::from_file(arg1.unwrap())?;
        if let MtxData::Sparse(size, coordinates, values, _) = sparse {
            println!("{:?}", size);
            println!("{:?}", coordinates.len());
            println!("{:?}", values.len());
            let num_entries = coordinates.len();

            let mut combined: Vec<([usize; 2], u32)> = coordinates.into_iter().zip(values.into_iter()).collect();

            // sort into row-major order
            combined.sort_by(|a, b| a.0[0].cmp(&b.0[0]).then(a.0[1].cmp(&b.0[1])));

            let (coordinates_sorted, values_sorted): (Vec<_>, Vec<_>) = combined.into_iter().unzip();

            println!("{:?}", coordinates_sorted.len());
            println!("{:?}", values_sorted.len());

            let mut count0 = 0;
            let mut count1 = 0;
            let mut count2 = 0;
            let mut count3 = 0;
            for idx in 0..num_entries {
                // println!("{:?} {}", coordinates_sorted[idx], values_sorted[idx]);
                if coordinates_sorted[idx][0] == 0 {
                    count0 += 1;
                }
                if coordinates_sorted[idx][0] == 1 {
                    count1 += 1;
                }
                if coordinates_sorted[idx][0] == 2 {
                    count2 += 1;
                }
                if coordinates_sorted[idx][0] == 3 {
                    count3 += 1;
                }
                if coordinates_sorted[idx][0] > 3 {
                    break;
                }
            }
            println!("{}", count0);
            println!("{}", count1);
            println!("{}", count2);
            println!("{}", count3);
        }
    }
    else {
        eprintln!("No file arg provided");
    }
    Ok(())
}
