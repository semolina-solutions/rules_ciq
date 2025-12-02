//! Resizes an image to fit within specified dimensions while preserving aspect ratio.
//!
//! Usage:
//!   scale_image <input_path> <output_path> <width> <height>
//!
//! This tool takes an input image and scales it so that it fits within the
//! given `width` and `height`.
//!
//! - If one dimension is 0, the image is scaled based on the other dimension.
//! - If both dimensions are non-zero, the image is scaled to fill the target
//!   dimensions while maintaining its aspect ratio (cropping may occur when
//!   rendered on the device).
//!
//! The output image is saved to `output_path`.

use clap::Parser;
use image::imageops::FilterType;
use image::GenericImageView;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    input_path: PathBuf,
    output_path: PathBuf,
    width: u32,
    height: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let img = image::open(&args.input_path)?;
    let (old_width, old_height) = img.dimensions();

    let mut width = args.width;
    let mut height = args.height;

    if width != 0 && height != 0 {
        // Scale the image to fill based on the aspect ratio of the original image.
        if (width as f64 / old_width as f64) < (height as f64 / old_height as f64) {
            width = 0;
        } else {
            height = 0;
        }
    }

    if width == 0 {
        width = ((height as f64 * old_width as f64) / old_height as f64).round() as u32;
    } else if height == 0 {
        height = ((width as f64 * old_height as f64) / old_width as f64).round() as u32;
    }

    let scaled_img = img.resize_exact(width, height, FilterType::Lanczos3);

    scaled_img.save(&args.output_path)?;

    Ok(())
}
