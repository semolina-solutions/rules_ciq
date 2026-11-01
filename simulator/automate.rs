use anyhow::{bail, Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::sync::Notify;
use tokio::time::timeout;

use lib::{stream_logs, LogLine, Shell};

const SHELL_RETRY_DELAY_MS: u64 = 250;
const INITIAL_APP_SETTLE_MS: u64 = 1500;

const SECTION_SETTINGS: u32 = 0x5E771465;
const STRING_BLOCK_SENTINEL: u32 = 0xABCDABCD;
const DATA_BLOCK_SENTINEL: u32 = 0xDA7ADA7A;

const TYPE_NULL: u8 = 0x00;
const TYPE_INT: u8 = 0x01;
const TYPE_FLOAT: u8 = 0x02;
const TYPE_STRING: u8 = 0x03;
const TYPE_BOOL: u8 = 0x09;
const TYPE_HASH: u8 = 0x0B;
const TYPE_LONG: u8 = 0x0E;
const TYPE_DOUBLE: u8 = 0x0F;

#[derive(Parser, Debug, Clone)]
pub struct AutomateArgs {
    #[arg(long)]
    pub ensure_linux_sdk: PathBuf,

    #[arg(long)]
    pub hermetic_ciq_dir: PathBuf,

    #[arg(long)]
    pub dockerfile: PathBuf,

    #[arg(long)]
    pub hook_src: PathBuf,

    #[arg(long)]
    pub application_id: Option<String>,

    #[arg(long)]
    pub manifest_xml: Option<PathBuf>,

    #[arg(long)]
    pub prg: PathBuf,

    #[arg(long)]
    pub debug_xml: PathBuf,

    #[arg(long)]
    pub settings_json: PathBuf,

    #[arg(long)]
    pub device: String,

    #[arg(long)]
    pub output: PathBuf,

    #[arg(long)]
    pub log_output: PathBuf,

    #[arg(long)]
    pub layout: Option<String>,

    #[arg(long)]
    pub background_color: Option<String>,

    #[arg(long)]
    pub time_display: Option<String>,

    #[arg(long)]
    pub units_display: Option<String>,

    #[arg(long)]
    pub language: Option<String>,

    #[arg(long, default_value_t = false)]
    pub simulate_activity: bool,

    #[arg(long)]
    pub fit_file: Option<PathBuf>,

    #[arg(long = "timer-action")]
    pub timer_actions: Vec<String>,

    #[arg(long = "setting")]
    pub settings: Vec<String>,

    #[arg(long = "action")]
    pub actions: Vec<String>,

    #[arg(long)]
    pub wait_for_log: Option<String>,

    #[arg(long, default_value_t = 1500)]
    pub delay_ms: u64,

    #[arg(long, default_value_t = 120)]
    pub timeout_secs: u64,
}

#[derive(Deserialize, Debug)]
struct SimulatorJson {
    display: DisplayConfig,
    #[serde(default)]
    keys: Vec<KeyConfig>,
}

#[derive(Deserialize, Debug)]
struct DisplayConfig {
    location: RectLocation,
}

#[derive(Deserialize, Debug)]
struct KeyConfig {
    id: String,
    #[serde(default)]
    behavior: Option<String>,
    #[serde(rename = "isHold", default)]
    is_hold: bool,
    location: RectLocation,
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct RectLocation {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
enum SettingValue {
    Null,
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool),
    Long(i64),
    Double(f64),
}

fn parse_setting_override(raw: &str) -> Result<(String, SettingValue)> {
    let (key, rhs) = raw
        .split_once('=')
        .with_context(|| format!("Invalid setting '{}': expected KEY=VALUE or KEY=TYPE:VALUE", raw))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("Setting key cannot be empty in '{}'", raw);
    }

    if let Some((type_prefix, val_str)) = rhs.split_once(':') {
        match type_prefix.to_ascii_lowercase().as_str() {
            "null" => return Ok((key, SettingValue::Null)),
            "bool" | "boolean" => {
                let b = match val_str.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    _ => bail!("Invalid boolean setting value '{}' for key '{}'", val_str, key),
                };
                return Ok((key, SettingValue::Bool(b)));
            }
            "int" | "i32" | "number" => {
                let v: i32 = val_str
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid int setting '{}' for key '{}'", val_str, key))?;
                return Ok((key, SettingValue::Int(v)));
            }
            "long" | "i64" => {
                let v: i64 = val_str
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid long setting '{}' for key '{}'", val_str, key))?;
                return Ok((key, SettingValue::Long(v)));
            }
            "float" | "f32" => {
                let v: f32 = val_str
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid float setting '{}' for key '{}'", val_str, key))?;
                return Ok((key, SettingValue::Float(v)));
            }
            "double" | "f64" => {
                let v: f64 = val_str
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid double setting '{}' for key '{}'", val_str, key))?;
                return Ok((key, SettingValue::Double(v)));
            }
            "string" | "str" => {
                return Ok((key, SettingValue::String(val_str.to_string())));
            }
            _ => {
                // Not a recognized type prefix (e.g. a string containing a colon like "https://...");
                // fall through to auto-inference on the full `rhs`.
            }
        }
    }

    let trimmed = rhs.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Ok((key, SettingValue::Bool(true)))
    } else if trimmed.eq_ignore_ascii_case("false") {
        Ok((key, SettingValue::Bool(false)))
    } else if trimmed.eq_ignore_ascii_case("null") {
        Ok((key, SettingValue::Null))
    } else if let Ok(i) = trimmed.parse::<i32>() {
        Ok((key, SettingValue::Int(i)))
    } else if let Ok(f) = trimmed.parse::<f32>() {
        if trimmed.contains('.') {
            Ok((key, SettingValue::Float(f)))
        } else {
            Ok((key, SettingValue::String(rhs.to_string())))
        }
    } else {
        Ok((key, SettingValue::String(rhs.to_string())))
    }
}

fn extract_settings_section_from_prg(prg_bytes: &[u8]) -> Option<&[u8]> {
    let mut pos = 0;
    while pos + 8 <= prg_bytes.len() {
        let sec_type = u32::from_be_bytes(prg_bytes[pos..pos + 4].try_into().ok()?);
        let sec_len = u32::from_be_bytes(prg_bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        pos += 8;
        if pos + sec_len > prg_bytes.len() {
            break;
        }
        if sec_type == SECTION_SETTINGS {
            return Some(&prg_bytes[pos..pos + sec_len]);
        }
        pos += sec_len;
    }
    None
}

fn read_string_at_offset(string_block: &[u8], offset: usize) -> Result<String> {
    if offset + 2 > string_block.len() {
        bail!("String offset {} out of bounds", offset);
    }
    let len_with_nul = u16::from_be_bytes([string_block[offset], string_block[offset + 1]]) as usize;
    let start = offset + 2;
    let str_len = len_with_nul.saturating_sub(1);
    if start + str_len > string_block.len() {
        bail!("String at offset {} exceeds string block", offset);
    }
    Ok(String::from_utf8_lossy(&string_block[start..start + str_len]).to_string())
}

fn parse_set_bytes(bytes: &[u8]) -> Result<Vec<(String, SettingValue)>> {
    let mut pos = 0;
    let mut string_block: &[u8] = &[];
    let mut data_block: &[u8] = &[];

    while pos + 8 <= bytes.len() {
        let sentinel = u32::from_be_bytes(bytes[pos..pos + 4].try_into()?);
        let len = u32::from_be_bytes(bytes[pos + 4..pos + 8].try_into()?) as usize;
        pos += 8;
        if pos + len > bytes.len() {
            break;
        }
        if sentinel == STRING_BLOCK_SENTINEL {
            string_block = &bytes[pos..pos + len];
        } else if sentinel == DATA_BLOCK_SENTINEL {
            data_block = &bytes[pos..pos + len];
        }
        pos += len;
    }

    let mut entries = Vec::new();
    if data_block.len() < 5 || data_block[0] != TYPE_HASH {
        return Ok(entries);
    }
    let num_entries = u32::from_be_bytes(data_block[1..5].try_into()?) as usize;
    let mut dpos = 5;
    for _ in 0..num_entries {
        if dpos + 5 > data_block.len() || data_block[dpos] != TYPE_STRING {
            break;
        }
        let key_off = u32::from_be_bytes(data_block[dpos + 1..dpos + 5].try_into()?) as usize;
        let key = read_string_at_offset(string_block, key_off)?;
        dpos += 5;

        if dpos >= data_block.len() {
            break;
        }
        let val_type = data_block[dpos];
        dpos += 1;
        let val = match val_type {
            TYPE_NULL => SettingValue::Null,
            TYPE_INT => {
                if dpos + 4 > data_block.len() {
                    break;
                }
                let v = i32::from_be_bytes(data_block[dpos..dpos + 4].try_into()?);
                dpos += 4;
                SettingValue::Int(v)
            }
            TYPE_FLOAT => {
                if dpos + 4 > data_block.len() {
                    break;
                }
                let v = f32::from_be_bytes(data_block[dpos..dpos + 4].try_into()?);
                dpos += 4;
                SettingValue::Float(v)
            }
            TYPE_STRING => {
                if dpos + 4 > data_block.len() {
                    break;
                }
                let off = u32::from_be_bytes(data_block[dpos..dpos + 4].try_into()?) as usize;
                dpos += 4;
                SettingValue::String(read_string_at_offset(string_block, off)?)
            }
            TYPE_BOOL => {
                if dpos + 1 > data_block.len() {
                    break;
                }
                let b = data_block[dpos] != 0;
                dpos += 1;
                SettingValue::Bool(b)
            }
            TYPE_LONG => {
                if dpos + 8 > data_block.len() {
                    break;
                }
                let v = i64::from_be_bytes(data_block[dpos..dpos + 8].try_into()?);
                dpos += 8;
                SettingValue::Long(v)
            }
            TYPE_DOUBLE => {
                if dpos + 8 > data_block.len() {
                    break;
                }
                let v = f64::from_be_bytes(data_block[dpos..dpos + 8].try_into()?);
                dpos += 8;
                SettingValue::Double(v)
            }
            _ => break,
        };
        entries.push((key, val));
    }
    Ok(entries)
}

fn serialize_set_bytes(entries: &[(String, SettingValue)]) -> Vec<u8> {
    let mut string_block = Vec::new();
    let mut string_offsets: HashMap<String, u32> = HashMap::new();

    let mut intern_string = |s: &str| -> u32 {
        if let Some(&off) = string_offsets.get(s) {
            return off;
        }
        let off = string_block.len() as u32;
        let bytes = s.as_bytes();
        let len_with_nul = (bytes.len() + 1) as u16;
        string_block.extend_from_slice(&len_with_nul.to_be_bytes());
        string_block.extend_from_slice(bytes);
        string_block.push(0);
        string_offsets.insert(s.to_string(), off);
        off
    };

    let mut data_block = Vec::new();
    data_block.push(TYPE_HASH);
    data_block.extend_from_slice(&(entries.len() as u32).to_be_bytes());

    for (key, val) in entries {
        let key_off = intern_string(key);
        data_block.push(TYPE_STRING);
        data_block.extend_from_slice(&key_off.to_be_bytes());

        match val {
            SettingValue::Null => {
                data_block.push(TYPE_NULL);
            }
            SettingValue::Int(v) => {
                data_block.push(TYPE_INT);
                data_block.extend_from_slice(&v.to_be_bytes());
            }
            SettingValue::Float(v) => {
                data_block.push(TYPE_FLOAT);
                data_block.extend_from_slice(&v.to_be_bytes());
            }
            SettingValue::String(s) => {
                let val_off = intern_string(s);
                data_block.push(TYPE_STRING);
                data_block.extend_from_slice(&val_off.to_be_bytes());
            }
            SettingValue::Bool(b) => {
                data_block.push(TYPE_BOOL);
                data_block.push(if *b { 1 } else { 0 });
            }
            SettingValue::Long(v) => {
                data_block.push(TYPE_LONG);
                data_block.extend_from_slice(&v.to_be_bytes());
            }
            SettingValue::Double(v) => {
                data_block.push(TYPE_DOUBLE);
                data_block.extend_from_slice(&v.to_be_bytes());
            }
        }
    }

    let mut out = Vec::new();
    if !string_block.is_empty() {
        out.extend_from_slice(&STRING_BLOCK_SENTINEL.to_be_bytes());
        out.extend_from_slice(&(string_block.len() as u32).to_be_bytes());
        out.extend_from_slice(&string_block);
    }
    out.extend_from_slice(&DATA_BLOCK_SENTINEL.to_be_bytes());
    out.extend_from_slice(&(data_block.len() as u32).to_be_bytes());
    out.extend_from_slice(&data_block);
    out
}

fn build_merged_set_bytes(prg_path: &Path, setting_args: &[String]) -> Result<Vec<u8>> {
    let prg_bytes = fs::read(prg_path).context("Failed to read PRG file")?;
    let mut entries = if let Some(sec) = extract_settings_section_from_prg(&prg_bytes) {
        parse_set_bytes(sec).unwrap_or_default()
    } else {
        Vec::new()
    };

    for raw in setting_args {
        let (key, val) = parse_setting_override(raw)?;
        if let Some(existing) = entries.iter_mut().find(|(k, _)| k == &key) {
            existing.1 = val;
        } else {
            entries.push((key, val));
        }
    }

    Ok(serialize_set_bytes(&entries))
}

fn resolve_docker_bin() -> PathBuf {
    let exe_name = if cfg!(windows) { "docker.exe" } else { "docker" };
    let mut search_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if !cfg!(windows) {
        search_dirs.push(PathBuf::from("/opt/homebrew/bin"));
        search_dirs.push(PathBuf::from("/usr/local/bin"));
        search_dirs.push(PathBuf::from("/usr/bin"));
    }
    for dir in search_dirs {
        let candidate = dir.join(exe_name);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(exe_name)
}

fn augmented_path_env() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    if cfg!(windows) {
        current
    } else if current.is_empty() {
        "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin".to_string()
    } else {
        format!("{}:/opt/homebrew/bin:/usr/local/bin", current)
    }
}

fn detect_docker_host_sock() -> Option<String> {
    if std::env::var_os("DOCKER_HOST").is_some() || Path::new("/var/run/docker.sock").exists() {
        return None;
    }
    let mut homes: Vec<PathBuf> = Vec::new();
    if let Some(h) = std::env::var_os("HOME") {
        homes.push(PathBuf::from(h));
    }
    if let Ok(entries) = fs::read_dir("/Users") {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && !homes.contains(&p) {
                homes.push(p);
            }
        }
    }
    let rel_candidates = [
        ".colima/default/docker.sock",
        ".docker/run/docker.sock",
        ".orbstack/run/docker.sock",
        ".rd/docker.sock",
    ];
    for home in homes {
        for rel in rel_candidates {
            let sock = home.join(rel);
            if sock.exists() {
                return Some(format!("unix://{}", sock.display()));
            }
        }
    }
    None
}

fn docker_std_command() -> std::process::Command {
    let mut cmd = std::process::Command::new(resolve_docker_bin());
    cmd.env("PATH", augmented_path_env());
    if let Some(host) = detect_docker_host_sock() {
        cmd.env("DOCKER_HOST", host);
    }
    cmd
}

fn docker_tokio_command() -> Command {
    let mut cmd = Command::new(resolve_docker_bin());
    cmd.env("PATH", augmented_path_env());
    if let Some(host) = detect_docker_host_sock() {
        cmd.env("DOCKER_HOST", host);
    }
    cmd
}

fn wait_and_sync_custom_set_files_in_container(container_name: &str) {
    let script = r#"
        for _ in $(seq 1 30); do
            found=0
            for d in $(find /tmp /root -type d -iname 'SETTINGS' 2>/dev/null | grep -i 'GARMIN/APPS'); do
                for existing in "$d"/*.SET "$d"/*.set; do
                    if [ -f "$existing" ]; then
                        cp -f /work/custom.SET "$existing"
                        found=1
                    fi
                done
                cp -f /work/*.SET "$d/" 2>/dev/null || true
            done
            if [ "$found" = "1" ]; then
                break
            fi
            sleep 0.1
        done
    "#;
    let _ = docker_std_command()
        .args(["exec", container_name, "bash", "-c", script])
        .status();
}

fn resolve_language_ini_index(lang: &str) -> Option<u32> {
    match lang.trim().to_ascii_lowercase().as_str() {
        "ces" | "czech" => Some(1),
        "dan" | "danish" => Some(2),
        "dut" | "nld" | "dutch" => Some(3),
        "eng" | "english" => Some(4),
        "fin" | "finnish" => Some(5),
        "fre" | "fra" | "french" => Some(6),
        "deu" | "ger" | "german" => Some(7),
        "gre" | "ell" | "greek" => Some(8),
        "hun" | "hungarian" => Some(9),
        "hrv" | "croatian" => Some(10),
        "ita" | "italian" => Some(11),
        "nob" | "nor" | "norwegian" => Some(12),
        "pol" | "polish" => Some(13),
        "por" | "portuguese" => Some(14),
        "rus" | "russian" => Some(15),
        "slo" | "slk" | "slovak" => Some(16),
        "slv" | "slovenian" => Some(17),
        "spa" | "spanish" => Some(18),
        "swe" | "swedish" => Some(19),
        "zht" | "traditional chinese" | "chinese (traditional)" => Some(20),
        "zhs" | "simplified chinese" | "chinese (simplified)" => Some(21),
        "jpn" | "japanese" => Some(22),
        "tha" | "thai" => Some(23),
        "ind" | "bahasa indonesia" | "indonesian" => Some(24),
        "zsm" | "bahasa malaysia" | "standard malay" | "malay" => Some(25),
        "kor" | "korean" => Some(26),
        "tur" | "turkish" => Some(27),
        "heb" | "hebrew" => Some(28),
        "ara" | "arabic" => Some(29),
        "bul" | "bulgarian" => Some(30),
        "vie" | "vietnamese" => Some(31),
        "est" | "estonian" => Some(32),
        "lav" | "latvian" => Some(33),
        "lit" | "lithuanian" => Some(34),
        "ron" | "rum" | "romanian" => Some(35),
        "ukr" | "ukrainian" => Some(36),
        _ => None,
    }
}

fn resolve_language_menu_label(lang: &str) -> &str {
    match lang.trim().to_ascii_lowercase().as_str() {
        "ara" | "arabic" => "Arabic",
        "bul" | "bulgarian" => "Bulgarian",
        "zhs" | "simplified chinese" => "Simplified Chinese",
        "zht" | "traditional chinese" => "Traditional Chinese",
        "hrv" | "croatian" => "Croatian",
        "ces" | "czech" => "Czech",
        "dan" | "danish" => "Danish",
        "dut" | "nld" | "dutch" => "Dutch",
        "eng" | "english" => "English",
        "est" | "estonian" => "Estonian",
        "fin" | "finnish" => "Finnish",
        "fre" | "fra" | "french" => "French",
        "deu" | "ger" | "german" => "German",
        "gre" | "ell" | "greek" => "Greek",
        "heb" | "hebrew" => "Hebrew",
        "hun" | "hungarian" => "Hungarian",
        "ind" | "bahasa indonesia" | "indonesian" => "Bahasa Indonesia",
        "ita" | "italian" => "Italian",
        "jpn" | "japanese" => "Japanese",
        "kor" | "korean" => "Korean",
        "lav" | "latvian" => "Latvian",
        "lit" | "lithuanian" => "Lithuanian",
        "zsm" | "standard malay" | "malay" => "Standard Malay",
        "nob" | "nor" | "norwegian" => "Norwegian",
        "pol" | "polish" => "Polish",
        "por" | "portuguese" => "Portuguese",
        "ron" | "rum" | "romanian" => "Romanian",
        "rus" | "russian" => "Russian",
        "slo" | "slk" | "slovak" => "Slovak",
        "slv" | "slovenian" => "Slovenian",
        "spa" | "spanish" => "Spanish",
        "swe" | "swedish" => "Swedish",
        "tha" | "thai" => "Thai",
        "tur" | "turkish" => "Turkish",
        "ukr" | "ukrainian" => "Ukrainian",
        "vie" | "vietnamese" => "Vietnamese",
        _ => lang.trim(),
    }
}

fn find_key<'a>(sim_json: &'a SimulatorJson, query: &str) -> Result<&'a KeyConfig> {
    let q = query.trim();
    let q_lower = q.to_ascii_lowercase();
    let canonical_id = match q_lower.as_str() {
        "select" | "start" => "enter",
        "back" => "esc",
        other => other,
    };
    for k in &sim_json.keys {
        if k.id.eq_ignore_ascii_case(q) || k.id.eq_ignore_ascii_case(canonical_id) {
            return Ok(k);
        }
        if let Some(beh) = &k.behavior {
            if beh.eq_ignore_ascii_case(q) {
                return Ok(k);
            }
        }
    }
    let available: Vec<String> = sim_json
        .keys
        .iter()
        .map(|k| match &k.behavior {
            Some(b) => format!("{} ({})", k.id, b),
            None => k.id.clone(),
        })
        .collect();
    bail!(
        "Button '{}' not found in device simulator.json. Available buttons: {}",
        query,
        available.join(", ")
    )
}

fn parse_display_coord(raw: &str, axis_size: f64) -> Result<f64> {
    let trimmed = raw.trim();
    if let Some(pct_str) = trimmed.strip_suffix('%') {
        let pct: f64 = pct_str
            .trim()
            .parse()
            .with_context(|| format!("Invalid percentage coordinate '{}'", raw))?;
        Ok((pct / 100.0) * axis_size)
    } else {
        let px: f64 = trimmed
            .parse()
            .with_context(|| format!("Invalid pixel coordinate '{}'", raw))?;
        Ok(px)
    }
}

fn generate_hook_commands(args: &AutomateArgs, sim_json: &SimulatorJson) -> Result<String> {
    let mut cmds = Vec::new();

    if let Some(lang) = &args.language {
        if resolve_language_ini_index(lang).is_none() {
            let label = resolve_language_menu_label(lang);
            cmds.push(format!("MENU /_Settings/Language/{}", label));
            if !label.eq_ignore_ascii_case("English") {
                cmds.push("WAIT 1500".to_string());
            }
        }
    }
    if let Some(layout) = &args.layout {
        cmds.push(format!("MENU /_Data Fields/Layout/{}", layout.trim()));
    }
    if let Some(bg) = &args.background_color {
        cmds.push(format!("MENU /_Data Fields/Background Color/{}", bg.trim()));
    }
    if let Some(td) = &args.time_display {
        cmds.push(format!("MENU /_Settings/Time Display/{}", td.trim()));
    }
    if let Some(ud) = &args.units_display {
        cmds.push(format!("MENU /_Settings/Units Display/{}", ud.trim()));
    }
    if !args.settings.is_empty() {
        cmds.push("MENU /_Settings/Trigger App Settings".to_string());
    }
    if args.fit_file.is_some() || !args.timer_actions.is_empty() {
        cmds.push("MENU /Si_mulation/Activity Data".to_string());
        cmds.push("WAIT 300".to_string());
        if let Some(fit_path) = &args.fit_file {
            let ext = fit_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "fit".to_string());
            cmds.push(format!("LOAD_FIT /work/activity.{}", ext));
            let has_explicit_play = args.timer_actions.iter().any(|a| {
                matches!(
                    a.trim().to_ascii_lowercase().as_str(),
                    "play" | "simulate"
                )
            });
            if !has_explicit_play {
                cmds.push("DIALOG_BUTTON __PLAY__".to_string());
                cmds.push("WAIT 300".to_string());
            }
        }
        for action in &args.timer_actions {
            let label = match action.trim().to_ascii_lowercase().as_str() {
                "start" => "Start",
                "pause" => "Pause",
                "resume" => "Resume",
                "lap" => "Lap",
                "workout_step" | "workout-step" | "workout step" => "Workout Step",
                "next_multisport" | "next-multisport" | "next multisport" => "Next Multisport",
                "save" => "Save",
                "discard" => "Discard",
                "play" | "simulate" => "__PLAY__",
                other => bail!(
                    "Unknown timer action '{}': expected start, pause, resume, lap, workout_step, next_multisport, save, discard, or play",
                    other
                ),
            };
            cmds.push(format!("DIALOG_BUTTON {}", label));
            cmds.push("WAIT 300".to_string());
        }
    }

    let disp = sim_json.display.location;
    for act in &args.actions {
        let (kind, param) = act.split_once(':').with_context(|| {
            format!(
                "Invalid action '{}': expected press:<btn>, hold:<btn>, tap:<x>,<y>, click:<x>,<y>, touch_hold:<x>,<y>, swipe:<dir|x1,y1,x2,y2>, drag:<x1,y1,x2,y2>, wait:<ms>, or menu:<path>",
                act
            )
        })?;
        match kind.trim().to_ascii_lowercase().as_str() {
            "press" => {
                let key = find_key(sim_json, param)?;
                let cx = key.location.x + key.location.width / 2.0;
                let cy = key.location.y + key.location.height / 2.0;
                let hold_ms = if key.is_hold { 1200 } else { 100 };
                cmds.push(format!("CLICK {:.1} {:.1} {}", cx, cy, hold_ms));
            }
            "hold" => {
                let key = find_key(sim_json, param)?;
                let cx = key.location.x + key.location.width / 2.0;
                let cy = key.location.y + key.location.height / 2.0;
                cmds.push(format!("CLICK {:.1} {:.1} 1200", cx, cy));
            }
            "tap" | "click" => {
                let (xs, ys) = param
                    .split_once(',')
                    .with_context(|| format!("Invalid {} coordinates '{}': expected x,y (pixels or %)", kind.trim(), param))?;
                let x = parse_display_coord(xs, disp.width)?;
                let y = parse_display_coord(ys, disp.height)?;
                cmds.push(format!("CLICK {:.1} {:.1} 100", disp.x + x, disp.y + y));
            }
            "touch_hold" | "touch-hold" => {
                let (xs, ys) = param
                    .split_once(',')
                    .with_context(|| format!("Invalid touch_hold coordinates '{}': expected x,y (pixels or %)", param))?;
                let x = parse_display_coord(xs, disp.width)?;
                let y = parse_display_coord(ys, disp.height)?;
                cmds.push(format!("CLICK {:.1} {:.1} 1200", disp.x + x, disp.y + y));
            }
            "swipe" | "drag" => {
                let (x1, y1, x2, y2) = match param.trim().to_ascii_lowercase().as_str() {
                    "left" => (
                        disp.x + disp.width * 0.8,
                        disp.y + disp.height * 0.5,
                        disp.x + disp.width * 0.2,
                        disp.y + disp.height * 0.5,
                    ),
                    "right" => (
                        disp.x + disp.width * 0.2,
                        disp.y + disp.height * 0.5,
                        disp.x + disp.width * 0.8,
                        disp.y + disp.height * 0.5,
                    ),
                    "up" => (
                        disp.x + disp.width * 0.5,
                        disp.y + disp.height * 0.8,
                        disp.x + disp.width * 0.5,
                        disp.y + disp.height * 0.2,
                    ),
                    "down" => (
                        disp.x + disp.width * 0.5,
                        disp.y + disp.height * 0.2,
                        disp.x + disp.width * 0.5,
                        disp.y + disp.height * 0.8,
                    ),
                    _ => {
                        let parts: Vec<&str> = param.split(',').collect();
                        if parts.len() != 4 {
                            bail!(
                                "Invalid {} parameter '{}': expected left, right, up, down, or x1,y1,x2,y2 (pixels or %)",
                                kind.trim(),
                                param
                            );
                        }
                        let sx1 = parse_display_coord(parts[0], disp.width)?;
                        let sy1 = parse_display_coord(parts[1], disp.height)?;
                        let sx2 = parse_display_coord(parts[2], disp.width)?;
                        let sy2 = parse_display_coord(parts[3], disp.height)?;
                        (disp.x + sx1, disp.y + sy1, disp.x + sx2, disp.y + sy2)
                    }
                };
                cmds.push(format!("SWIPE {:.1} {:.1} {:.1} {:.1} 360", x1, y1, x2, y2));
            }
            "wait" => {
                let ms: u64 = param.trim().parse()?;
                cmds.push(format!("WAIT {}", ms));
            }
            "menu" => {
                cmds.push(format!("MENU {}", param.trim()));
            }
            other => bail!("Unknown action type '{}' in '{}'", other, act),
        }
    }

    cmds.push(format!("WAIT {}", args.delay_ms));
    cmds.push("SAVE /work/output.png".to_string());

    Ok(cmds.join("\n") + "\n")
}

fn matches_log_pattern(logs: &str, pattern: &str) -> Result<bool> {
    if logs.contains(pattern) {
        return Ok(true);
    }
    let re = Regex::new(pattern)
        .with_context(|| format!("Invalid log regex pattern '{}'", pattern))?;
    Ok(re.is_match(logs))
}

fn fnv1a_hash(chunks: &[&[u8]]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for chunk in chunks {
        for &b in *chunk {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn ensure_docker_image(dockerfile: &Path, hook_src: &Path, lock_parent: &Path) -> Result<String> {
    let df_bytes = fs::read(dockerfile).context("Failed to read Dockerfile")?;
    let hook_bytes = fs::read(hook_src).context("Failed to read gtk_automation_hook.rs")?;
    let hash = fnv1a_hash(&[&df_bytes, &hook_bytes]);
    let image_tag = format!("rules-ciq-simulator:{:016x}", hash);

    let inspect_ok = docker_std_command()
        .args(["image", "inspect", &image_tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if inspect_ok {
        return Ok(image_tag);
    }

    let lock_dir = lock_parent.join(format!(".docker_build_{:016x}.lock", hash));
    while fs::create_dir(&lock_dir).is_err() {
        std::thread::sleep(Duration::from_millis(500));
    }
    struct LockGuard(PathBuf);
    impl Drop for LockGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir(&self.0);
        }
    }
    let _guard = LockGuard(lock_dir);

    let inspect_ok = docker_std_command()
        .args(["image", "inspect", &image_tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if inspect_ok {
        return Ok(image_tag);
    }

    let ctx_dir = std::env::temp_dir().join(format!("ciq_docker_ctx_{}_{:016x}", std::process::id(), hash));
    fs::create_dir_all(&ctx_dir)?;
    let ctx_dir = fs::canonicalize(&ctx_dir).unwrap_or(ctx_dir);
    fs::copy(dockerfile, ctx_dir.join("Dockerfile"))?;
    fs::copy(hook_src, ctx_dir.join("gtk_automation_hook.rs"))?;

    let ctx_dir_str = ctx_dir.to_string_lossy().replace('\\', "/");
    let status = docker_std_command()
        .args([
            "build",
            "--platform",
            "linux/amd64",
            "-t",
            &image_tag,
            &ctx_dir_str,
        ])
        .status()
        .context("Failed to invoke `docker build`")?;
    let _ = fs::remove_dir_all(&ctx_dir);

    if !status.success() {
        bail!("`docker build` failed for {}", image_tag);
    }

    Ok(image_tag)
}

fn ensure_linux_sdk_dir(ensure_script: &Path) -> Result<PathBuf> {
    let out = std::process::Command::new("bash")
        .arg(ensure_script)
        .output()
        .with_context(|| format!("Failed to execute {}", ensure_script.display()))?;
    if !out.status.success() {
        bail!(
            "Failed to ensure Linux SDK ({}):\n{}",
            ensure_script.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path_str.is_empty() {
        bail!("ensure_linux_sdk script returned empty path");
    }
    Ok(PathBuf::from(path_str))
}

struct ContainerGuard {
    container_name: String,
    work_dir: PathBuf,
    child: Option<std::process::Child>,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        let _ = docker_std_command()
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = fs::remove_dir_all(&self.work_dir);
    }
}

fn main() {
    let args = AutomateArgs::parse();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    if let Err(e) = rt.block_on(run_automation(args)) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

async fn run_automation(args: AutomateArgs) -> Result<()> {
    let raw_app_id = if let Some(id) = &args.application_id {
        id.clone()
    } else if let Some(manifest_path) = &args.manifest_xml {
        let xml_content = fs::read_to_string(manifest_path)
            .with_context(|| format!("Failed to read manifest XML {}", manifest_path.display()))?;
        let re = Regex::new(r#"<iq:application\b[^>]*\bid="([^"]+)""#)?;
        let caps = re
            .captures(&xml_content)
            .context("Could not find iq:application id attribute in manifest XML")?;
        caps[1].to_string()
    } else {
        bail!("Either --application-id or --manifest-xml must be provided");
    };

    let linux_sdk_dir = ensure_linux_sdk_dir(&args.ensure_linux_sdk)?;
    let image_tag = ensure_docker_image(&args.dockerfile, &args.hook_src, &args.hermetic_ciq_dir)?;

    let sim_json_path = args
        .hermetic_ciq_dir
        .join("Devices")
        .join(&args.device)
        .join("simulator.json");
    let sim_json_str = fs::read_to_string(&sim_json_path)
        .with_context(|| format!("Failed to read {}", sim_json_path.display()))?;
    let sim_json: SimulatorJson = serde_json::from_str(&sim_json_str)
        .with_context(|| format!("Failed to parse {}", sim_json_path.display()))?;

    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let unique_id = format!("ciq-auto-{}-{}", std::process::id(), now_nanos);
    let work_dir = std::env::temp_dir().join(&unique_id);
    fs::create_dir_all(&work_dir)?;
    let work_dir = fs::canonicalize(&work_dir).unwrap_or(work_dir);

    let prg_name = args
        .prg
        .file_name()
        .context("Invalid PRG path")?
        .to_string_lossy()
        .to_string();
    let prg_stem = args
        .prg
        .file_stem()
        .context("Invalid PRG stem")?
        .to_string_lossy()
        .to_string();
    let prg_stem_upper = prg_stem.to_uppercase();

    let settings_name = args
        .settings_json
        .file_name()
        .context("Invalid settings JSON path")?
        .to_string_lossy()
        .to_string();
    let settings_suffix = "-settings.json";
    let settings_adjusted_name = if settings_name.ends_with(settings_suffix) {
        let prefix_len = settings_name.len() - settings_suffix.len();
        let prefix = &settings_name[..prefix_len];
        format!("{}{}", prefix.to_uppercase(), &settings_name[prefix_len..])
    } else {
        settings_name.clone()
    };
    fs::copy(&args.settings_json, work_dir.join(&settings_adjusted_name))?;

    let debug_xml_adjusted_name = if args.debug_xml.exists() {
        let name = args
            .debug_xml
            .file_name()
            .context("Invalid debug XML path")?
            .to_string_lossy()
            .to_uppercase();
        fs::copy(&args.debug_xml, work_dir.join(&name))?;
        Some(name)
    } else {
        None
    };

    fs::copy(&args.prg, work_dir.join(&prg_name))?;

    let mut custom_set_filenames = Vec::new();
    if !args.settings.is_empty() {
        let set_bytes = build_merged_set_bytes(&args.prg, &args.settings)?;
        fs::write(work_dir.join("custom.SET"), &set_bytes)?;
        let exact_set_name = format!("{}.SET", prg_stem);
        fs::write(work_dir.join(&exact_set_name), &set_bytes)?;
        custom_set_filenames.push(exact_set_name.clone());
        let upper_set_name = format!("{}.SET", prg_stem_upper);
        if upper_set_name != exact_set_name {
            fs::write(work_dir.join(&upper_set_name), &set_bytes)?;
            custom_set_filenames.push(upper_set_name);
        }
    }

    if let Some(lang) = &args.language {
        if let Some(lang_idx) = resolve_language_ini_index(lang) {
            fs::write(work_dir.join("simulator.ini"), format!("Language={}\n", lang_idx))?;
        }
    }

    if let Some(fit_path) = &args.fit_file {
        let ext = fit_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "fit".to_string());
        let fname = format!("activity.{}", ext);
        fs::copy(fit_path, work_dir.join(&fname))
            .with_context(|| format!("Failed to copy fit file {}", fit_path.display()))?;
    }

    let hook_cmds = generate_hook_commands(&args, &sim_json)?;
    fs::write(work_dir.join("hook_cmds.txt"), hook_cmds)?;

    let linux_sdk_dir = fs::canonicalize(&linux_sdk_dir).unwrap_or(linux_sdk_dir);
    let devices_dir = fs::canonicalize(args.hermetic_ciq_dir.join("Devices"))
        .unwrap_or_else(|_| args.hermetic_ciq_dir.join("Devices"));
    let fonts_dir = fs::canonicalize(args.hermetic_ciq_dir.join("Fonts"))
        .unwrap_or_else(|_| args.hermetic_ciq_dir.join("Fonts"));

    let container_script = r#"
        set -e
        rm -f /tmp/.X99-lock
        mkdir -p /root/.Garmin/ConnectIQ
        if [ -f /work/simulator.ini ]; then
            cp /work/simulator.ini /root/.Garmin/ConnectIQ/simulator.ini
        fi
        Xvfb :99 -screen 0 1280x1024x24 >/dev/null 2>&1 &
        export DISPLAY=:99
        export HOME=/root
        export GDK_BACKEND=x11
        export LIBGL_ALWAYS_SOFTWARE=1
        export NO_AT_BRIDGE=1
        exec env LD_PRELOAD=/usr/local/lib/libciq_gtk_hook.so /ciq/sdk/bin/simulator >/work/simulator.log 2>&1
    "#;

    let linux_sdk_mount = format!("{}:/ciq/sdk:ro", linux_sdk_dir.to_string_lossy().replace('\\', "/"));
    let devices_mount = format!(
        "{}:/root/.Garmin/ConnectIQ/Devices:ro",
        devices_dir.to_string_lossy().replace('\\', "/")
    );
    let fonts_mount = format!(
        "{}:/root/.Garmin/ConnectIQ/Fonts:ro",
        fonts_dir.to_string_lossy().replace('\\', "/")
    );
    let work_mount = format!("{}:/work", work_dir.to_string_lossy().replace('\\', "/"));

    let child = docker_std_command()
        .args([
            "run",
            "--rm",
            "--platform",
            "linux/amd64",
            "--name",
            &unique_id,
            "-v",
            &linux_sdk_mount,
            "-v",
            &devices_mount,
            "-v",
            &fonts_mount,
            "-v",
            &work_mount,
            &image_tag,
            "bash",
            "-c",
            container_script,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start simulator Docker container")?;

    let _guard = ContainerGuard {
        container_name: unique_id.clone(),
        work_dir: work_dir.clone(),
        child: Some(child),
    };

    let overall_timeout = Duration::from_secs(args.timeout_secs);
    timeout(overall_timeout, async {
        let mut shell = loop {
            let mut cmd = docker_tokio_command();
            cmd.args(["exec", "-i", &unique_id, "/ciq/sdk/bin/shell"])
                .stderr(Stdio::null());
            if let Ok(mut s) = Shell::from_command(cmd).await {
                if s.wait_for(":>", Some(Duration::from_millis(SHELL_RETRY_DELAY_MS)))
                    .await
                    .is_ok()
                {
                    break s;
                }
            }
            tokio::time::sleep(Duration::from_millis(SHELL_RETRY_DELAY_MS)).await;
        };

        shell
            .send(&format!(
                "push \"/work/{}\" \"0:/GARMIN/Settings/{}\"",
                settings_adjusted_name, settings_adjusted_name
            ))
            .await?;
        shell.wait_for("File pushed successfully", None).await?;
        shell.wait_for(":>", None).await?;
        if settings_name != settings_adjusted_name {
            shell
                .send(&format!(
                    "push \"/work/{}\" \"0:/GARMIN/Settings/{}\"",
                    settings_adjusted_name, settings_name
                ))
                .await?;
            shell.wait_for("File pushed successfully", None).await?;
            shell.wait_for(":>", None).await?;
        }

        if let Some(dbg_name) = &debug_xml_adjusted_name {
            shell
                .send(&format!(
                    "push \"/work/{}\" \"0:/GARMIN/Debug/{}\"",
                    dbg_name, dbg_name
                ))
                .await?;
            shell.wait_for("File pushed successfully", None).await?;
            shell.wait_for(":>", None).await?;
        }

        shell
            .send(&format!(
                "push \"/work/{}\" \"0:/GARMIN/APPS/{}\"",
                prg_name, prg_name
            ))
            .await?;
        shell.wait_for("File pushed successfully", None).await?;
        shell.wait_for(":>", None).await?;

        for set_fname in &custom_set_filenames {
            shell
                .send(&format!(
                    "push \"/work/{}\" \"0:/GARMIN/APPS/SETTINGS/{}\"",
                    set_fname, set_fname
                ))
                .await?;
            shell.wait_for("File pushed successfully", None).await?;
            shell.wait_for(":>", None).await?;
        }

        if args.fit_file.is_none() && args.simulate_activity {
            shell.send("fit_parse --simulate").await?;
            shell.wait_for(":>", None).await?;
        }

        shell.send("ciq").await?;
        shell.wait_for("[1][0]shellConnected", None).await?;

        shell
            .send(&format!("[1][0]openDevice {}", args.device))
            .await?;
        shell
            .wait_for(&format!("[1][0]deviceStarted {}", args.device), None)
            .await?;

        let formatted_app_id = raw_app_id.replace('-', "").to_uppercase();
        shell
            .send(&format!("[2][0]startApp {}", formatted_app_id))
            .await?;

        if !custom_set_filenames.is_empty() {
            wait_and_sync_custom_set_files_in_container(&unique_id);
            shell
                .send(&format!("[2][0]startApp {}", formatted_app_id))
                .await?;
        }

        let app_logs = Arc::new(Mutex::new(String::new()));
        let log_notify = Arc::new(Notify::new());

        let logs_clone = Arc::clone(&app_logs);
        let notify_clone = Arc::clone(&log_notify);
        let app_id_clone = formatted_app_id.clone();
        let log_task = tokio::spawn(async move {
            let _ = stream_logs(shell, &app_id_clone, |log_line| {
                if let LogLine::App(msg) = log_line {
                    let mut buf = logs_clone.lock().unwrap();
                    buf.push_str(msg);
                    if !msg.ends_with('\n') {
                        buf.push('\n');
                    }
                    drop(buf);
                    notify_clone.notify_waiters();
                }
                false
            })
            .await;
        });

        if let Some(log_pattern) = &args.wait_for_log {
            loop {
                let matched = {
                    let current = app_logs.lock().unwrap();
                    matches_log_pattern(&current, log_pattern)?
                };
                if matched {
                    break;
                }
                log_notify.notified().await;
            }
            if !custom_set_filenames.is_empty() {
                wait_and_sync_custom_set_files_in_container(&unique_id);
            }
            fs::write(work_dir.join("trigger"), "1\n")?;
        } else {
            tokio::time::sleep(Duration::from_millis(INITIAL_APP_SETTLE_MS)).await;
            if !custom_set_filenames.is_empty() {
                wait_and_sync_custom_set_files_in_container(&unique_id);
            }
            fs::write(work_dir.join("trigger"), "1\n")?;
        }

        let done_path = work_dir.join("done");
        while !done_path.exists() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Allow any final in-flight application stdout lines to flush through `shell`.
        tokio::time::sleep(Duration::from_millis(150)).await;
        log_task.abort();

        let captured_logs = app_logs.lock().unwrap().clone();
        let captured_path = work_dir.join("output.png");
        if !captured_path.exists() {
            let sim_log = fs::read_to_string(work_dir.join("simulator.log")).unwrap_or_default();
            bail!(
                "Simulator automation finished without producing output.png. Simulator log:\n{}",
                sim_log
            );
        }

        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&captured_path, &args.output)
            .with_context(|| format!("Failed to copy screenshot to {}", args.output.display()))?;

        if let Some(parent) = args.log_output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&args.log_output, &captured_logs)
            .with_context(|| format!("Failed to write application log to {}", args.log_output.display()))?;

        Ok::<(), anyhow::Error>(())
    })
    .await
    .context("Timed out waiting for simulator automation")??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_binary_round_trip() {
        let entries = vec![
            ("myInt".to_string(), SettingValue::Int(42)),
            ("myBool".to_string(), SettingValue::Bool(true)),
            ("myStr".to_string(), SettingValue::String("hello".to_string())),
            ("myFloat".to_string(), SettingValue::Float(3.5)),
            ("myNull".to_string(), SettingValue::Null),
        ];
        let bytes = serialize_set_bytes(&entries);
        let parsed = parse_set_bytes(&bytes).expect("Failed to parse serialized .SET bytes");
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0].0, "myInt");
        assert!(matches!(parsed[0].1, SettingValue::Int(42)));
        assert_eq!(parsed[1].0, "myBool");
        assert!(matches!(parsed[1].1, SettingValue::Bool(true)));
        assert_eq!(parsed[2].0, "myStr");
        assert!(matches!(&parsed[2].1, SettingValue::String(s) if s == "hello"));
        assert_eq!(parsed[3].0, "myFloat");
        assert!(matches!(parsed[3].1, SettingValue::Float(f) if (f - 3.5).abs() < 1e-6));
        assert_eq!(parsed[4].0, "myNull");
        assert!(matches!(parsed[4].1, SettingValue::Null));
    }

    #[test]
    fn test_parse_setting_override() {
        let (k, v) = parse_setting_override("zones=int:5").unwrap();
        assert_eq!(k, "zones");
        assert!(matches!(v, SettingValue::Int(5)));

        let (k, v) = parse_setting_override("dark=true").unwrap();
        assert_eq!(k, "dark");
        assert!(matches!(v, SettingValue::Bool(true)));

        let (k, v) = parse_setting_override("url=https://example.com").unwrap();
        assert_eq!(k, "url");
        assert!(matches!(v, SettingValue::String(s) if s == "https://example.com"));
    }

    #[test]
    fn test_resolve_language_ini_index() {
        assert_eq!(resolve_language_ini_index("eng"), Some(4));
        assert_eq!(resolve_language_ini_index("English"), Some(4));
        assert_eq!(resolve_language_ini_index("deu"), Some(7));
        assert_eq!(resolve_language_ini_index("German"), Some(7));
        assert_eq!(resolve_language_ini_index("unknown"), None);
    }

    #[test]
    fn test_parse_display_coord() {
        assert!((parse_display_coord("208", 416.0).unwrap() - 208.0).abs() < 1e-6);
        assert!((parse_display_coord(" 130.5 ", 260.0).unwrap() - 130.5).abs() < 1e-6);
        assert!((parse_display_coord("50%", 416.0).unwrap() - 208.0).abs() < 1e-6);
        assert!((parse_display_coord("25%", 260.0).unwrap() - 65.0).abs() < 1e-6);
        assert!((parse_display_coord("12.5%", 400.0).unwrap() - 50.0).abs() < 1e-6);
        assert!(parse_display_coord("abc", 416.0).is_err());
        assert!(parse_display_coord("abc%", 416.0).is_err());
    }

    #[test]
    fn test_generate_hook_commands_with_pixel_and_percent_coords() {
        let sim_json = SimulatorJson {
            display: DisplayConfig {
                location: RectLocation {
                    x: 100.0,
                    y: 50.0,
                    width: 400.0,
                    height: 300.0,
                },
            },
            keys: vec![],
        };
        let args = AutomateArgs {
            ensure_linux_sdk: PathBuf::new(),
            hermetic_ciq_dir: PathBuf::new(),
            dockerfile: PathBuf::new(),
            hook_src: PathBuf::new(),
            application_id: None,
            manifest_xml: None,
            prg: PathBuf::new(),
            debug_xml: PathBuf::new(),
            settings_json: PathBuf::new(),
            device: "venu2".to_string(),
            output: PathBuf::new(),
            log_output: PathBuf::new(),
            layout: None,
            background_color: None,
            time_display: None,
            units_display: None,
            language: None,
            simulate_activity: false,
            fit_file: None,
            timer_actions: vec![],
            settings: vec![],
            actions: vec![
                "tap:200,150".to_string(),
                "click:50%,50%".to_string(),
                "touch_hold:25%,100".to_string(),
                "drag:80%,50%,20%,50%".to_string(),
                "swipe:40,30,360,270".to_string(),
            ],
            wait_for_log: None,
            delay_ms: 1500,
            timeout_secs: 120,
        };
        let script = generate_hook_commands(&args, &sim_json).unwrap();
        let lines: Vec<&str> = script.lines().collect();
        assert_eq!(
            lines,
            vec![
                "CLICK 300.0 200.0 100",
                "CLICK 300.0 200.0 100",
                "CLICK 200.0 150.0 1200",
                "SWIPE 420.0 200.0 180.0 200.0 360",
                "SWIPE 140.0 80.0 460.0 320.0 360",
                "WAIT 1500",
                "SAVE /work/output.png",
            ]
        );
    }
}
