use anyhow::{bail, Context, Result};
use clap::Parser;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug, Clone)]
pub struct VerifyScreenshotArgs {
    #[arg(long)]
    pub actual: PathBuf,

    #[arg(long)]
    pub expected: PathBuf,

    #[arg(long)]
    pub diff_output: Option<PathBuf>,

    #[arg(long, default_value_t = 0)]
    pub pixel_tolerance: u8,

    #[arg(long, default_value_t = 0)]
    pub max_diff_pixels: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenshotDiffStats {
    pub total_pixels: u64,
    pub diff_pixels: u64,
    pub max_channel_delta: u8,
}

pub fn compare_screenshots(
    actual_path: &Path,
    expected_path: &Path,
    pixel_tolerance: u8,
    max_diff_pixels: u64,
    diff_output: Option<&Path>,
) -> Result<ScreenshotDiffStats> {
    let actual_img = image::open(actual_path)
        .with_context(|| format!("Failed to open actual screenshot {}", actual_path.display()))?
        .to_rgba8();
    let expected_img = image::open(expected_path)
        .with_context(|| format!("Failed to open expected screenshot {}", expected_path.display()))?
        .to_rgba8();

    let (w, h) = actual_img.dimensions();
    let (ew, eh) = expected_img.dimensions();
    if (w, h) != (ew, eh) {
        bail!(
            "Screenshot dimensions mismatch: actual is {}x{} ({}), expected is {}x{} ({})",
            w,
            h,
            actual_path.display(),
            ew,
            eh,
            expected_path.display()
        );
    }

    let total_pixels = (w as u64) * (h as u64);
    let mut diff_pixels: u64 = 0;
    let mut max_channel_delta: u8 = 0;
    let mut diff_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let ap = actual_img.get_pixel(x, y);
            let ep = expected_img.get_pixel(x, y);
            let mut px_delta: u8 = 0;
            for c in 0..4 {
                let d = ap[c].abs_diff(ep[c]);
                if d > px_delta {
                    px_delta = d;
                }
            }
            if px_delta > max_channel_delta {
                max_channel_delta = px_delta;
            }
            if px_delta > pixel_tolerance {
                diff_pixels += 1;
                diff_img.put_pixel(x, y, Rgba([255, 0, 64, 255]));
            } else {
                diff_img.put_pixel(x, y, Rgba([ap[0] / 3, ap[1] / 3, ap[2] / 3, 255]));
            }
        }
    }

    if diff_pixels > max_diff_pixels {
        let mut diff_note = String::new();
        if let Some(diff_path) = diff_output {
            if let Some(parent) = diff_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if diff_img.save(diff_path).is_ok() {
                diff_note = format!(" (visual diff written to {})", diff_path.display());
            }
        }
        let pct = if total_pixels > 0 {
            (diff_pixels as f64) * 100.0 / (total_pixels as f64)
        } else {
            0.0
        };
        bail!(
            "Screenshot diff failed against {}: {} / {} pixels ({:.3}%) differed by more than tolerance {} (max allowed differing pixels: {}, max channel delta: {}){}",
            expected_path.display(),
            diff_pixels,
            total_pixels,
            pct,
            pixel_tolerance,
            max_diff_pixels,
            max_channel_delta,
            diff_note
        );
    }

    Ok(ScreenshotDiffStats {
        total_pixels,
        diff_pixels,
        max_channel_delta,
    })
}

fn main() {
    let args = VerifyScreenshotArgs::parse();
    let result = (|| -> Result<()> {
        let undeclared_dir = std::env::var("TEST_UNDECLARED_OUTPUTS_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let diff_out = args
            .diff_output
            .clone()
            .or_else(|| undeclared_dir.as_ref().map(|d| d.join("diff.png")));
        compare_screenshots(
            &args.actual,
            &args.expected,
            args.pixel_tolerance,
            args.max_diff_pixels,
            diff_out.as_deref(),
        )?;
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_screenshots_diff_and_tolerance() {
        let dir = std::env::temp_dir().join(format!("ciq_diff_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let p1 = dir.join("img1.png");
        let p2 = dir.join("img2.png");
        let diff_p = dir.join("diff.png");

        let img1: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(4, 4, Rgba([10, 20, 30, 255]));
        let mut img2 = img1.clone();
        img2.put_pixel(1, 1, Rgba([14, 20, 30, 255]));

        img1.save(&p1).unwrap();
        img2.save(&p2).unwrap();

        let strict_res = compare_screenshots(&p1, &p2, 0, 0, Some(&diff_p));
        assert!(strict_res.is_err());
        assert!(diff_p.exists());

        let tol_stats = compare_screenshots(&p1, &p2, 5, 0, None).unwrap();
        assert_eq!(tol_stats.diff_pixels, 0);
        assert_eq!(tol_stats.max_channel_delta, 4);

        let max_px_stats = compare_screenshots(&p1, &p2, 0, 1, None).unwrap();
        assert_eq!(max_px_stats.diff_pixels, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}

