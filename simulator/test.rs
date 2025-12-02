//! Runs the ConnectIQ simulator in test mode for a specific application.
//!
//! Usage:
//!   test <simulator_path> <shell_path> <application_id> <prg_path> <settings_json_path> <device>
//!
//! This tool performs the following steps:
//! 1. Starts the ConnectIQ simulator.
//! 2. Pushes the application settings (JSON) to the simulator.
//! 3. Pushes the application executable (PRG) to the simulator.
//! 4. Triggers the "Run No Evil" tests for the application.
//! 5. Streams logs and monitors for test results.
//! 6. Exits with 0 if all tests pass, or 1 otherwise.

use lib::{run_simulator_operation, test};

fn main() {
    run_simulator_operation(test);
}
