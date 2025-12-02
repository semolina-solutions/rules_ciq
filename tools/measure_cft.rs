//! Measures the height of characters in a ConnectIQ Custom Font (CFT) file.
//!
//! Usage:
//!   measure_cft <font_path>
//!
//! This tool reads the header of a `.cft` file to extract the font height.
//! It expects the height to be a 16-bit big-endian integer located at offset 22.

use clap::Parser;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
struct Args {
    font_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let font_path = &args.font_path;

    let mut file = match File::open(font_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file '{}': {}", font_path.display(), e);
            process::exit(1);
        }
    };

    if let Err(e) = file.seek(SeekFrom::Start(22)) {
        eprintln!("Error seeking in file: {}", e);
        process::exit(1);
    }

    let mut height_data = [0u8; 2];
    if let Err(e) = file.read_exact(&mut height_data) {
        eprintln!("Error reading from file: {}", e);
        process::exit(1);
    }

    let height = u16::from_be_bytes(height_data);
    println!("{}", height);
}
