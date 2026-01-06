//! Resizes an image to fit within specified dimensions while preserving aspect ratio.
//!
//! Usage:
//!   bazel run @rules_ciq//tools:scale_image -- <input_path> <output_path> [--width <width>] [--height <height>]
//!
//! This tool takes an input image and scales it so that it fits within the
//! given `width` and/or `height`.
//!
//! - If only one dimension is specified, the image is scaled based on that dimension
//!   while preserving aspect ratio.
//! - If both dimensions are specified, the image is scaled to fit within the
//!   target dimensions while maintaining its aspect ratio (cropping may occur).
//! - If neither dimension is specified, an error is returned.
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

    #[clap(long)]
    width: Option<u32>,

    #[clap(long)]
    height: Option<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.width.is_none() && args.height.is_none() {
        return Err("At least one of --width or --height must be specified".into());
    }

    let img = image::open(&args.input_path)?;
    let (old_width, old_height) = img.dimensions();

    let mut width = args.width;
    let mut height = args.height;

    // If both dimensions are provided, determine which one is the constraining dimension
    // and unset the other so it can be recalculated based on aspect ratio.
    if let (Some(w), Some(h)) = (width, height) {
        if (w as f64 / old_width as f64) < (h as f64 / old_height as f64) {
            width = None;
        } else {
            height = None;
        }
    }

    // Calculate the missing dimension
    if width.is_none() {
        let h = height.unwrap();
        width = Some(((h as f64 * old_width as f64) / old_height as f64).round() as u32);
    } else if height.is_none() {
        let w = width.unwrap();
        height = Some(((w as f64 * old_height as f64) / old_width as f64).round() as u32);
    }

    let scaled_img = img.resize_exact(width.unwrap(), height.unwrap(), FilterType::Lanczos3);

    scaled_img.save(&args.output_path)?;

    Ok(())
}
