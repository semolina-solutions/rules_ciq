//! Downloads files from an MTP device.
//!
//! Usage:
//!   bazel run @rules_ciq//mtp:download <device_path> <local_path> [<device_path> <local_path> ...]
//!
//! Example:
//!   bazel run @rules_ciq//mtp:download /GARMIN/Apps/my_app.prg ./my_app.prg
//!
//! This tool takes pairs of arguments: a source file path on the MTP device
//! the device to the specified local paths.
//!
//! If the device path is prefixed with `?` (e.g. `?/GARMIN/Apps/my_app.prg`),
//! the download will "soft-fail": if the file does not exist on the device,
//! the tool will exit successfully without downloading anything.

use lib::{download_file, run_mtp_operation};

fn main() {
    run_mtp_operation(download_file);
}
