//! Lookups the Garmin Product ID for a given Hardware Part Number.
//!
//! Usage:
//!   bazel run @rules_ciq//metrics:get_product_id -- <hardware_part_number> --products-json-paths <path>...
//!
//! This tool searches through the provided list of product JSON files to find the matching
//! product ID for the given Hardware Part Number (HPN).
//!
//! The found Product ID is printed to stdout.

use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Hardware Part Number to look up
    hardware_part_number: String,

    /// Paths to product JSON files
    #[arg(long, num_args = 1..)]
    products_json_paths: Vec<PathBuf>,
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let hpn = &args.hardware_part_number;

    // Normalize HPN
    let norm_hpn = if let Some(idx) = hpn.rfind('-') {
        format!("{}-00", &hpn[..idx])
    } else {
        hpn.to_string()
    };

    let mut product_id: Option<i64> = None;

    for path in &args.products_json_paths {
        let content = fs::read_to_string(path)?;
        // Parse as generic Value first to handle both Arrays (lists) and Objects (batches)
        let json: Value = serde_json::from_str(&content)?;

        if let Some(products) = json.as_array() {
            for p in products {
                if let Some(pn) = p.get("partNumber").and_then(|s| s.as_str()) {
                    // Check direct match
                    if pn == hpn {
                        product_id = p.get("productId").and_then(|v| {
                            v.as_i64()
                                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                        });
                        break;
                    }

                    // Check normalized match
                    if let Some(idx) = pn.rfind('-') {
                        let p_norm = format!("{}-00", &pn[..idx]);
                        if p_norm == norm_hpn {
                            product_id = p.get("productId").and_then(|v| {
                                v.as_i64()
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                            });
                        }
                    }
                }
            }
        }
        if product_id.is_some() {
            break;
        }
    }

    if let Some(pid) = product_id {
        println!("{}", pid);
        Ok(())
    } else {
        anyhow::bail!(
            "Product ID not found for HPN {} (normalized {})",
            hpn,
            norm_hpn
        )
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Warning: {}. Using dummy Product ID 0.", e);
        println!("0");
    }
}
