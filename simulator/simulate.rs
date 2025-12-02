//! Runs the ConnectIQ simulator for a specific application.
//!
//! Usage:
//!   simulate <simulator_path> <shell_path> <application_id> <prg_path> <settings_json_path> <device>
//!
//! This tool performs the following steps:
//! 1. Starts the ConnectIQ simulator.
//! 2. Pushes the application settings (JSON) to the simulator.
//! 3. Pushes the application executable (PRG) to the simulator.
//! 4. Starts the application on the specified device.
//! 5. Streams logs from the simulator to stdout, filtering and formatting them.
//!
//! On macOS, it also attempts to bring the simulator window to the foreground.

use lib::{run_simulator_operation, simulate};

fn main() {
    run_simulator_operation(simulate);
}
