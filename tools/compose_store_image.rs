//! Composes a store listing image from one or more input images.
//!
//! If multiple images are provided, it creates an animated GIF.
//! If a single image is provided, it outputs it in the desired format.
//! It attempts to optimize the file size to fit within a specified limit (filesize)
//! by reducing color depth or scaling down the image.
//!
//! Usage:
//!   bazel run @rules_ciq//tools:compose_store_image -- --output-path <output_path> --max-size-kb <size> [--transition-millis <millis>...] <image_paths>...

use clap::Parser;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, DynamicImage, Frame, GenericImageView, ImageFormat};
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    output_path: PathBuf,

    #[arg(long)]
    max_size_kb: u64,

    #[arg(long, num_args(1..))]
    transition_millis: Vec<u64>,

    #[arg(required = true)]
    image_paths: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.image_paths.is_empty() {
        return Err("No images provided".into());
    }

    let images = load_images(&args.image_paths)?;

    // Try with original scale, reducing color depth.
    // Bits per channel: 8 (original), 6, 5, 4
    let bit_depths = [8u8, 6, 5, 4];

    for &bits in &bit_depths {
        let posterized_images = if bits == 8 {
            images.clone()
        } else {
            posterize_images(&images, bits)
        };

        let result = generate_output(&posterized_images, 1.0, &args)?;
        let size_kb = result.len() as u64 / 1024;

        if size_kb <= args.max_size_kb {
            std::fs::write(&args.output_path, &result)?;
            return Ok(());
        }
    }

    // Binary search for best scale. Stick to original colors for scaling to
    // keep it simple, as scaling destroys quality anyway.
    let mut best_result = None;
    let mut low = 0.05;
    let mut high = 1.0;
    // 8 iterations is a precision of ~0.4%
    for _ in 0..8 {
        let scale = (low + high) / 2.0;
        let result = generate_output(&images, scale, &args)?;
        let size_kb = result.len() as u64 / 1024;

        if size_kb <= args.max_size_kb {
            best_result = Some(result);
            low = scale;
        } else {
            high = scale;
        }
    }

    if let Some(result) = best_result {
        std::fs::write(&args.output_path, &result)?;
    } else {
        // If even the lowest scale doesn't fit, just save the smallest generation.
        let final_try = generate_output(&images, low, &args)?;
        std::fs::write(&args.output_path, &final_try)?;
        println!(
            "Warning: Could not meet size requirement even at {:.2} scale.",
            low
        );
    }

    Ok(())
}

fn generate_output(
    images: &[DynamicImage],
    scale: f64,
    args: &Args,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let current_images = if (scale - 1.0).abs() > f64::EPSILON {
        resize_images(images, scale)
    } else {
        images.to_vec()
    };

    if current_images.len() > 1 {
        create_gif(&current_images, &args.transition_millis)
    } else {
        create_single_image(&current_images[0], &args.output_path)
    }
}

fn load_images(paths: &[PathBuf]) -> Result<Vec<DynamicImage>, Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    for path in paths {
        images.push(image::open(path)?);
    }
    Ok(images)
}

fn resize_images(images: &[DynamicImage], scale: f64) -> Vec<DynamicImage> {
    images
        .iter()
        .map(|img| {
            let (w, h) = img.dimensions();
            let new_w = (w as f64 * scale) as u32;
            let new_h = (h as f64 * scale) as u32;
            img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
        })
        .collect()
}

fn posterize_images(images: &[DynamicImage], bits: u8) -> Vec<DynamicImage> {
    let mask = !((1u8 << (8 - bits)) - 1);
    images
        .iter()
        .map(|img| {
            let mut rgba = img.to_rgba8();
            for pixel in rgba.pixels_mut() {
                pixel[0] &= mask;
                pixel[1] &= mask;
                pixel[2] &= mask;
                // Force alpha to opaque (255) to reduce palette usage as
                // transparency is not needed
                pixel[3] = 255;
            }
            DynamicImage::ImageRgba8(rgba)
        })
        .collect()
}

fn create_gif(
    images: &[DynamicImage],
    transition_millis: &[u64],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let delays = if transition_millis.is_empty() {
        vec![1000; images.len()]
    } else if transition_millis.len() == 1 {
        vec![transition_millis[0]; images.len()]
    } else if transition_millis.len() == images.len() {
        transition_millis.to_vec()
    } else {
        return Err(format!(
            "Number of transition durations ({}) does not match number of images ({})",
            transition_millis.len(),
            images.len()
        )
        .into());
    };

    let mut buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut buf);
        encoder.set_repeat(Repeat::Infinite)?;

        for (i, img) in images.iter().enumerate() {
            let delay = Delay::from_saturating_duration(Duration::from_millis(delays[i]));

            let frame = Frame::from_parts(img.to_rgba8(), 0, 0, delay);
            encoder.encode_frame(frame)?;
        }
    }
    Ok(buf)
}

fn create_single_image(
    img: &DynamicImage,
    output_path: &PathBuf,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Cursor::new(Vec::new());
    let format = ImageFormat::from_path(output_path).unwrap_or(ImageFormat::Png);
    img.write_to(&mut buf, format)?;
    Ok(buf.into_inner())
}
