//! Uploads files to an MTP device.
//!
//! Usage:
//!   upload <local_path> <device_path> [<local_path> <device_path> ...]
//!
//! Example:
//!   upload ./my_app.prg /GARMIN/Apps/my_app.prg
//!
//! This tool takes pairs of arguments: a source file path on the local
//! the local filesystem to the specified paths on the device.
//!
//! If the device path is prefixed with `?` (e.g. `?/GARMIN/Apps/my_app.prg`),
//! the upload will "soft-fail": if the file already exists on the device, it
//! will NOT be replaced, and the tool will exit successfully.

use lib::{run_mtp_operation, upload_file};

fn main() {
    run_mtp_operation(upload_file);
}
