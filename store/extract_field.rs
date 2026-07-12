//! Extracts a data field rectangle from a simulator screenshot.
//!
//! Produces an output image with the same pixel dimensions as the input.
//! Pixels inside the specified rectangle are copied from the input; all other
//! pixels are fully transparent (RGBA alpha = 0).
//!
//! Usage:
//!   extract_field <input_path> <output_path> --x <X> --y <Y> --width <W> --height <H>

use clap::Parser;
use image::{ImageBuffer, RgbaImage};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    input_path: PathBuf,
    output_path: PathBuf,

    /// Left edge of the field rectangle (pixels from left of screen).
    #[clap(long)]
    x: u32,

    /// Top edge of the field rectangle (pixels from top of screen).
    #[clap(long)]
    y: u32,

    /// Width of the field rectangle in pixels.
    #[clap(long)]
    width: u32,

    /// Height of the field rectangle in pixels.
    #[clap(long)]
    height: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input = image::open(&args.input_path)?.to_rgba8();
    let (img_w, img_h) = input.dimensions();

    // Clamp the rectangle to the image bounds.
    let x_end = (args.x + args.width).min(img_w);
    let y_end = (args.y + args.height).min(img_h);

    // Start with a fully-transparent canvas the same size as the input.
    let mut output: RgbaImage = ImageBuffer::new(img_w, img_h);

    // Copy only the pixels inside the field rectangle.
    for y in args.y..y_end {
        for x in args.x..x_end {
            output.put_pixel(x, y, *input.get_pixel(x, y));
        }
    }

    output.save(&args.output_path)?;
    Ok(())
}
