//! Scales a numeric value by a percentage.
//!
//! Usage:
//!   bazel run @rules_ciq//tools:scale_value -- <value> <percent> [--snap <snap>]
//!
//! This tool calculates `value * percent / 100` and rounds the result to the
//! nearest multiple of `snap`.
//!
//! Arguments:
//! - `value`: The initial value (integer).
//! - `percent`: The percentage to scale by (integer).
//! - `--snap`: The snapping interval (integer, default: 1).
//!
//! The result is printed to stdout.

use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The value to scale
    value: u32,
    /// The percentage to scale by
    percent: u32,
    /// The snapping interval (default: 1)
    #[clap(long, default_value = "1")]
    snap: u32,
}

fn main() {
    let args = Args::parse();

    // Scale the value
    let scaled = (args.value as f64 * args.percent as f64) / 100.0;

    // Round to nearest snap
    let snap = args.snap as f64;
    let rounded = (scaled / snap).round() * snap;

    println!("{}", rounded as u32);
}
