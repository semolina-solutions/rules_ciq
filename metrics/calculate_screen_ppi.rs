//! Calculates the screen pixels per inch (PPI) for a given Garmin product.
//!
//! Usage:
//!   bazel run @rules_ciq//metrics:calculate_screen_ppi -- <product_id> --product-data-json-paths <path>...
//!
//! This tool searches through the provided list of product JSON files to find the matching
//! product details (using product_id) and screen specifications (display size and resolution).
//!
//! The calculated PPI is printed to stdout as an integer.
//!

use clap::Parser;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Product ID to calculate PPI for
    product_id: String,

    /// Paths to product JSON files
    #[arg(long, num_args = 1..)]
    product_data_json_paths: Vec<PathBuf>,
}

fn clean_text(text: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    let cleaned = re.replace_all(text, " ");
    let cleaned = cleaned.replace("&Prime;", "\"");
    let cleaned = cleaned.replace("&quot;", "\"");
    let cleaned = cleaned.replace("&nbsp;", " ");
    let cleaned = cleaned.replace("”", "\"");
    let cleaned = cleaned.replace("“", "\"");
    let cleaned = cleaned.replace("″", "\"");
    cleaned.trim().to_string()
}

fn parse_res(res_raw: &str) -> Option<(f64, f64)> {
    // Remove (RGB) etc
    let re_paren = Regex::new(r"\(.*?\)").unwrap();
    let res_clean = re_paren.replace_all(res_raw, "");

    // 1. Standard W x H
    let re_wxh = Regex::new(r"(\d+)\s*[xX]\s*(\d+)").unwrap();
    if let Some(captures) = re_wxh.captures(&res_clean) {
        let w = captures[1].parse::<f64>().ok()?;
        let h = captures[2].parse::<f64>().ok()?;
        return Some((w, h));
    }

    // 2. Diameter resolution (e.g. "240 pixel diameter")
    // Assume square/circular
    let re_diam = Regex::new(r"(?i)(\d+)\s*pixels?\s*diameter").unwrap();
    if let Some(captures) = re_diam.captures(&res_clean) {
        let d = captures[1].parse::<f64>().ok()?;
        return Some((d, d));
    }

    None
}

fn parse_size(size_raw: &str, w_px: f64, h_px: f64) -> Option<f64> {
    let diag_px = (w_px.powi(2) + h_px.powi(2)).sqrt();

    // Helper to convert to inches
    let to_inches = |val: f64, unit: &str| -> f64 {
        let unit = unit.to_lowercase();
        if unit == "mm" {
            val / 25.4
        } else if unit == "cm" {
            val / 2.54
        } else {
            val
        }
    };

    let unit_pattern = r#"(”|″|\u{201d}|"|in|inch|inches|mm|cm)"#;

    // 1. Diameter (Tight match)
    let re_diameter = Regex::new(&format!(
        r#"(?i)(\d+(?:\.\d+)?)\s*{}.*?diamet(?:er|re)"#,
        unit_pattern
    ))
    .unwrap();
    if let Some(captures) = re_diameter.captures(size_raw) {
        if let Ok(d) = captures[1].parse::<f64>() {
            let d_inches = to_inches(d, &captures[2]);
            if d_inches > 0.0 {
                return Some(w_px / d_inches);
            }
        }
    }

    // 2. Diagonal (Tight match)
    let re_diag = Regex::new(&format!(r#"(?i)(\d+(?:\.\d+)?)\s*{}.*?diag"#, unit_pattern)).unwrap();
    if let Some(captures) = re_diag.captures(size_raw) {
        if let Ok(d) = captures[1].parse::<f64>() {
            let d_inches = to_inches(d, &captures[2]);
            if d_inches > 0.0 {
                return Some(diag_px / d_inches);
            }
        }
    }

    // Fallback for implicit diagonal if it's the only dimension pattern at start
    let re_implicit = Regex::new(&format!(r#"(?i)^(\d+(?:\.\d+)?)\s*{}$"#, unit_pattern)).unwrap();
    if let Some(captures) = re_implicit.captures(size_raw) {
        if let Ok(d) = captures[1].parse::<f64>() {
            let d_inches = to_inches(d, &captures[2]);
            if d_inches > 0.0 {
                return Some(diag_px / d_inches);
            }
        }
    }

    // Fallback for leading dimension with extra text (e.g. '2.6" colour')
    let re_leading = Regex::new(r#"(?i)^(\d+(?:\.\d+)?)\s*([”″\u{201d}"a-zA-Z]+)"#).unwrap();
    if let Some(captures) = re_leading.captures(size_raw) {
        if let Ok(d) = captures[1].parse::<f64>() {
            let d_inches = to_inches(d, &captures[2]);
            if d_inches > 0.0 {
                return Some(diag_px / d_inches);
            }
        }
    }

    // 3. W x H (Allow quote AND W/H e.g. 4.1"W)
    let re_wxh = Regex::new(&format!(r#"(?i)(\d+(?:\.\d+)?)\s*{}?\s*(?:W|width)?\s*[xX]\s*(\d+(?:\.\d+)?)\s*{}?\s*(?:H|height)?"#, unit_pattern, unit_pattern)).unwrap();

    if let Some(captures) = re_wxh.captures(size_raw) {
        if let (Ok(w), Ok(h)) = (captures[1].parse::<f64>(), captures[3].parse::<f64>()) {
            // Handle units for W and H if present
            let u1 = captures.get(2).map_or("", |m| m.as_str());
            let u2 = captures.get(4).map_or("", |m| m.as_str());

            // Logic to determine effective units
            let u1_def = if u1.is_empty() { "inch" } else { u1 };
            let u2_def = if u2.is_empty() { "inch" } else { u2 };

            let u1_eff = if u1.is_empty() && !u2.is_empty() {
                u2
            } else {
                u1_def
            };
            let u2_eff = if u2.is_empty() && !u1.is_empty() {
                u1
            } else {
                u2_def
            };

            let w_val = if w > 20.0 && u1_eff == "inch" {
                w / 25.4
            } else {
                to_inches(w, u1_eff)
            };
            let h_val = if h > 20.0 && u2_eff == "inch" {
                h / 25.4
            } else {
                to_inches(h, u2_eff)
            };

            if w_val > 0.0 && h_val > 0.0 {
                let d = (w_val.powi(2) + h_val.powi(2)).sqrt();
                if d > 0.0 {
                    return Some(diag_px / d);
                }
            }
        }
    }

    None
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let product_id = args.product_id;

    // Find Specs in product_data batch files
    let mut size_raw = String::new();
    let mut res_raw = String::new();
    let mut found_specs = false;

    let target_specs = [
        "productSpecPhysicalDisplaySize",
        "productSpecPhysicalDisplayResolution",
    ];

    // Walk provided files for specs
    for path in &args.product_data_json_paths {
        // Filter for English files only to ensure regex compatibility
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            if !fname.starts_with("en-") {
                continue;
            }
        }

        let content = fs::read_to_string(path)?;
        let detail: Value = serde_json::from_str(&content)?;

        if let Some(groups) = detail
            .get("productSpecs")
            .and_then(|ps| ps.get("specGroups"))
            .and_then(|sg| sg.as_array())
        {
            for group in groups {
                if let Some(specs) = group.get("specs").and_then(|s| s.as_array()) {
                    for spec in specs {
                        if let Some(key) = spec.get("specKey").and_then(|k| k.as_str()) {
                            if target_specs.contains(&key) {
                                // Check values for our PID
                                if let Some(values) = spec.get("values").and_then(|v| v.as_array())
                                {
                                    for val in values {
                                        let val_pid_str = val.get("pid").and_then(|p| {
                                            p.as_i64()
                                                .map(|i| i.to_string())
                                                .or(p.as_str().map(|s| s.to_string()))
                                        });
                                        if val_pid_str == Some(product_id.clone()) {
                                            let display_val = val
                                                .get("specDisplayValue")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("");
                                            if key == "productSpecPhysicalDisplaySize" {
                                                size_raw = display_val.to_string();
                                            } else if key == "productSpecPhysicalDisplayResolution"
                                            {
                                                res_raw = display_val.to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !size_raw.is_empty() && !res_raw.is_empty() {
            found_specs = true;
            break;
        }
    }

    if !found_specs {
        anyhow::bail!("Specs not found for PID {}", product_id);
    }

    // Sanitize and parse
    let size_clean = clean_text(&size_raw);
    let res_clean = clean_text(&res_raw);

    if size_clean.to_lowercase().contains("not applicable") {
        anyhow::bail!("Size not applicable");
    }

    if let Some((w_px, h_px)) = parse_res(&res_clean) {
        if let Some(ppi) = parse_size(&size_clean, w_px, h_px) {
            println!("{}", ppi.round() as i64);
            return Ok(());
        }
    }

    anyhow::bail!(
        "Failed to calculate PPI from Size: '{}', Res: '{}'",
        size_clean,
        res_clean
    );
}

fn main() {
    if let Err(e) = run() {
        let err_msg = e.to_string();
        // If it's a parsing error (regex mismatch), we want to fail hard to catch bugs.
        // For missing products/specs (legacy devices), we fallback to 200 PPI.
        if err_msg.contains("Failed to calculate PPI") {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        } else {
            eprintln!("Warning: {}. Using fallback PPI 200.", e);
            println!("200");
        }
    }
}
