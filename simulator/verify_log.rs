use anyhow::{bail, Context, Result};
use clap::Parser;
use regex::Regex;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
pub struct VerifyLogArgs {
    #[arg(long)]
    pub log: PathBuf,

    #[arg(long = "expect-log")]
    pub expect_logs: Vec<String>,

    #[arg(long = "forbid-log")]
    pub forbid_logs: Vec<String>,
}

pub fn matches_log_pattern(logs: &str, pattern: &str) -> Result<bool> {
    if logs.contains(pattern) {
        return Ok(true);
    }
    let re = Regex::new(pattern)
        .with_context(|| format!("Invalid log regex pattern '{}'", pattern))?;
    Ok(re.is_match(logs))
}

pub fn verify_logs(app_logs: &str, expect_logs: &[String], forbid_logs: &[String]) -> Result<()> {
    for expected in expect_logs {
        if !matches_log_pattern(app_logs, expected)? {
            bail!(
                "Expected log pattern '{}' was not found in application logs.\n--- Captured Application Logs ---\n{}",
                expected,
                app_logs.trim_end()
            );
        }
    }

    for forbidden in forbid_logs {
        if matches_log_pattern(app_logs, forbidden)? {
            bail!(
                "Forbidden log pattern '{}' was found in application logs.\n--- Captured Application Logs ---\n{}",
                forbidden,
                app_logs.trim_end()
            );
        }
    }

    Ok(())
}

fn main() {
    let args = VerifyLogArgs::parse();
    let result = (|| -> Result<()> {
        let content = fs::read_to_string(&args.log)
            .with_context(|| format!("Failed to read log file {}", args.log.display()))?;
        verify_logs(&content, &args.expect_logs, &args.forbid_logs)
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
    fn test_verify_logs_substring_and_regex() {
        let sample_logs = "\
interactive_app ready: BEREIT [24H/METR]
interactive_app action=NEXT page=2 select=0 history=NEXT
interactive_app action=SEL page=2 select=1 history=NEXT>SEL
data_field compute: title=ZONE 4 targetHr=165 state=RUNNING laps=1 hr=129
";

        verify_logs(
            sample_logs,
            &[
                "BEREIT [24H/METR]".to_string(),
                "history=NEXT>SEL".to_string(),
                r"hr=[1-9]\d+".to_string(),
            ],
            &["READY [12H/STAT]".to_string(), "state=PAUSED".to_string()],
        )
        .expect("Expected log assertions to pass");

        let missing_err = verify_logs(sample_logs, &["history=SWIPE_L".to_string()], &[]);
        assert!(missing_err.is_err());

        let forbid_err = verify_logs(sample_logs, &[], &["state=RUNNING".to_string()]);
        assert!(forbid_err.is_err());
    }
}

