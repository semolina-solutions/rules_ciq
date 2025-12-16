use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Stdio;
use std::time::Duration;
use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

const SHELL_RETRY_DELAY_MS: u64 = 250;

#[derive(Parser, Clone)]
pub struct Args {
    pub simulator_path: PathBuf,
    pub shell_path: PathBuf,
    pub application_id: String,
    pub prg_path: PathBuf,
    pub debug_xml_path: PathBuf,
    pub settings_json_path: PathBuf,
    pub device: String,
}

pub struct Shell {
    _process: Child,
    pub stdout_reader: BufReader<tokio::process::ChildStdout>,
    pub stdin_writer: tokio::process::ChildStdin,
}

impl Shell {
    pub async fn new(shell_path: &Path) -> Result<Self> {
        let mut process = Command::new(shell_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn shell process")?;

        let stdout = process
            .stdout
            .take()
            .context("Failed to capture shell stdout")?;
        let stdin = process
            .stdin
            .take()
            .context("Failed to capture shell stdin")?;

        let stdout_reader = BufReader::new(stdout);

        Ok(Self {
            _process: process,
            stdout_reader,
            stdin_writer: stdin,
        })
    }

    pub async fn send(&mut self, command: &str) -> Result<()> {

        self.stdin_writer
            .write_all(format!("{}\n", command).as_bytes())
            .await
            .context("Failed to write to shell stdin")?;
        self.stdin_writer
            .flush()
            .await
            .context("Failed to flush shell stdin")?;
        Ok(())
    }

    pub async fn wait_for(&mut self, phrase: &str, timeout_duration: Option<Duration>) -> Result<()> {


        let wait_future = async {
            let phrase_bytes = phrase.as_bytes();
            let mut buffer = Vec::new();
            let mut byte = [0u8; 1];

            loop {
                let n = self.stdout_reader.read(&mut byte).await?;
                if n == 0 {
                    return Err(anyhow::anyhow!("Stream ended before finding phrase"));
                }

                buffer.push(byte[0]);


                // Check if buffer ends with the phrase
                if buffer.len() >= phrase_bytes.len() {
                    let start = buffer.len() - phrase_bytes.len();
                    if &buffer[start..] == phrase_bytes {

                        return Ok(());
                    }
                }
            }
        };

        if let Some(duration) = timeout_duration {
            timeout(duration, wait_future)
                .await
                .context("Timed out waiting for phrase")??;
        } else {
            wait_future.await?;
        }

        Ok(())
    }
}

pub fn run_simulator_operation<F, Fut>(operation: F)
where
    F: FnOnce(Shell, Args) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let args = Args::parse();
    
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let result = rt.block_on(async {
        // Start simulator
        // Has no effect if simulator is already running.
        let mut simulator_process = std::process::Command::new(&args.simulator_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn simulator")?;

        // Retry connecting to shell until successful
        let mut shell = loop {
            if let Ok(mut s) = Shell::new(&args.shell_path).await {
                // Try to get the initial prompt
                if s.wait_for(":>", Some(Duration::from_millis(SHELL_RETRY_DELAY_MS))).await.is_ok() {
                    break s;
                }
            }
            tokio::time::sleep(Duration::from_millis(SHELL_RETRY_DELAY_MS)).await;
        };

        // Push settings
        let settings_name = args
            .settings_json_path
            .file_name()
            .context("Invalid settings path")?
            .to_string_lossy();
        
        let settings_suffix = "-settings.json";
        let settings_adjusted_name = if settings_name.ends_with(settings_suffix) {
            let prefix_len = settings_name.len() - settings_suffix.len();
            let prefix = &settings_name[..prefix_len];
            format!("{}{}", prefix.to_uppercase(), &settings_name[prefix_len..])
        } else {
            settings_name.to_string()
        };

        shell
            .send(&format!(
                "push \"{}\" \"0:/GARMIN/Settings/{}\"",
                args.settings_json_path.display(),
                settings_adjusted_name
            ))
            .await?;
        shell.wait_for("File pushed successfully", None).await?;
        shell.wait_for(":>", None).await?;

        // Push Debug XML
        if args.debug_xml_path.exists() {
            let debug_xml_name = args
                .debug_xml_path
                .file_name()
                .context("Invalid Debug XML path")?
                .to_string_lossy();
            
            shell
                .send(&format!(
                    "push \"{}\" \"0:/GARMIN/Debug/{}\"",
                    args.debug_xml_path.display(),
                    debug_xml_name.to_uppercase()
                ))
                .await?;
            shell.wait_for("File pushed successfully", None).await?;
            shell.wait_for(":>", None).await?;
        }

        // Push PRG
        let prg_name = args
            .prg_path
            .file_name()
            .context("Invalid PRG path")?
            .to_string_lossy();
        
        shell
            .send(&format!(
                "push \"{}\" \"0:/GARMIN/APPS/{}\"",
                args.prg_path.display(),
                prg_name
            ))
            .await?;
        shell.wait_for("File pushed successfully", None).await?;
        shell.wait_for(":>", None).await?;

        // Connect to CIQ
        shell.send("ciq").await?;
        shell.wait_for("[1][0]shellConnected", None).await?;

        // Open Device
        shell
            .send(&format!("[1][0]openDevice {}", args.device))
            .await?;
        shell
            .wait_for(&format!("[1][0]deviceStarted {}", args.device), None)
            .await?;

        let result = operation(shell, args.clone()).await;

        // Cleanup (optional, as OS will clean up, but good practice)
        let _ = simulator_process.kill();

        result
    });

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
        process::exit(1);
    }
}

pub async fn simulate(mut shell: Shell, args: Args) -> Result<()> {
    // Start App
    let formatted_app_id = args.application_id.replace("-", "").to_uppercase();
    shell
        .send(&format!("[2][0]startApp {}", formatted_app_id))
        .await?;

    // On macOS, bring the simulator window to the foreground using AppleScript
    // This preserves the process handle so we can terminate it later
    #[cfg(target_os = "macos")]
    {
        // Use AppleScript to activate the ConnectIQ app
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"ConnectIQ\" to activate")
            .output();
    }

    let mut on_new_line = true;
    stream_logs(shell, &formatted_app_id, |log_line| {
        if matches!(log_line, LogLine::Test(_)) {
            return false;
        }

        let output = match log_line {
            LogLine::Simulator(c) => format!("[SIMULATOR] {}\n", c),
            LogLine::Device(c) => format!("[DEVICE] {}\n", c),
            LogLine::App(c) => c.to_string(),
            LogLine::Raw(c) => c.to_string(),
            LogLine::Test(_) => unreachable!(),
        };

        if !matches!(log_line, LogLine::App(_)) && !on_new_line {
            println!();
        }

        print!("{}", output);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        on_new_line = output.ends_with('\n');
        false
    }).await
}

pub async fn test(mut shell: Shell, args: Args) -> Result<()> {
    // Start App (Run All Tests)
    let formatted_app_id = args.application_id.replace("-", "").to_uppercase();
    shell
        .send(&format!("[2][0]runAllTests {}", formatted_app_id))
        .await?;

    let mut test_outcome = None;

    stream_logs(shell, &formatted_app_id, |log_line| {
        match log_line {
            LogLine::App(message) => {
                print!("{}", message);
            }
            LogLine::Test(message) => {
                print!("{}", message);
                test_outcome = Some(message.to_string());
            }
            LogLine::Simulator(message) => {
                if message.contains("deviceTerminated") {
                    return true;
                }
            }
            _ => {}
        }
        false
    }).await?;

    if let Some(text) = test_outcome {
        let re = Regex::new(r"PASSED \(passed=\d+, failed=0, errors=0\)").unwrap();
        if !re.is_match(&text) {
            return Err(anyhow::anyhow!("Not all tests passed"));
        }
    } else {
            return Err(anyhow::anyhow!("No test outcome found"));
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum LogLine<'a> {
    Simulator(&'a str),
    Device(&'a str),
    App(&'a str),
    Test(&'a str),
    Raw(&'a str),
}

async fn stream_logs<F>(shell: Shell, formatted_app_id: &str, mut callback: F) -> Result<()> 
where F: FnMut(LogLine) -> bool
{
    let mut lines = shell.stdout_reader.lines();
    let simulator_message_prefix = "[1][0]";
    let device_message_prefix = "[2][0]";
    let app_message_prefix = format!("[3][{}][0]", formatted_app_id);
    let test_message_prefix = format!("[4][{}][0]", formatted_app_id);

    while let Some(line) = lines.next_line().await?.map(|l| l.replace("\\n", "\n")) {
        let log_line = if let Some(stripped) = line.strip_prefix(simulator_message_prefix) {
            LogLine::Simulator(stripped)
        } else if let Some(stripped) = line.strip_prefix(device_message_prefix) {
            LogLine::Device(stripped)
        } else if let Some(stripped) = line.strip_prefix(&app_message_prefix) {
            LogLine::App(stripped)
        } else if let Some(stripped) = line.strip_prefix(&test_message_prefix) {
            LogLine::Test(stripped)
        } else {
            LogLine::Raw(&line)
        };

        if callback(log_line) {
            break;
        }
    }

    Ok(())
}
