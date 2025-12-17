//! Interprets a profiling log file (.PRF).
//!
//! Usage:
//!   bazel run @rules_ciq//device:interpret_profiling_log -- <path_to_prf> [--debug-xml <path_to_debug_xml>] [--show-callstacks]
//!
//! This tool analyzes a Connect IQ profiling log file and produces a statistical performance report.
//! It can optionally resolve function names and source locations using the associated debug XML file.

use anyhow::{Context, Result};
use clap::Parser;
use prost::bytes::Buf; // Use Buf trait for advancing through the slice
use prost::Message;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use xml::reader::{EventReader, XmlEvent};

// Manual Protobuf Definition
#[derive(Clone, PartialEq, Message)]
pub struct ProfileEvent {
    /// Timestamp in microseconds
    #[prost(int64, optional, tag = "1")]
    pub timestamp: Option<i64>,

    /// Program Counter (points to function/instruction)
    #[prost(int32, optional, tag = "2")]
    pub pc: Option<i32>,

    /// Event type: 2 = Function Enter, 3 = Function Exit
    #[prost(enumeration = "EventType", optional, tag = "3")]
    pub event_type: Option<i32>,

    /// For Enter events: The caller PC
    /// For Exit events: 0 (or absent)
    #[prost(int32, optional, tag = "4")]
    pub extra_data: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum EventType {
    Unknown = 0,
    Enter = 2,
    Exit = 3,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the .PRF profiling file
    file: String,

    /// Path to the .prg.debug.xml file for symbol resolution (optional)
    #[arg(short, long)]
    debug_xml: Option<String>,

    /// Show unique call stacks for each function
    #[arg(long)]
    show_callstacks: bool,
}

#[derive(Clone, Debug)]
struct SourceLocation {
    file: String,
    line: i32,
    symbol: String,
}

struct FunctionStats {
    name: String,
    call_count: u64,
    total_time_us: u64,
    actual_time_us: u64,
    call_stacks: HashMap<Vec<i32>, u64>, // Maps unique stack trace (list of PCs) to count
}

struct StackFrame {
    pc: i32,
    start_time: i64,
    children_time: i64,
}

struct DebugInfo {
    pc_to_name: HashMap<i32, String>,
    pc_to_source: HashMap<i32, SourceLocation>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Parse Debug XML if provided
    let debug_info = if let Some(xml_path) = &args.debug_xml {
        Some(parse_debug_xml(xml_path)?)
    } else {
        None
    };

    let empty_map = HashMap::new();
    let pc_map = debug_info
        .as_ref()
        .map(|d| &d.pc_to_name)
        .unwrap_or(&empty_map);

    let mut file = File::open(&args.file).context("Failed to open PRF file")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .context("Failed to read PRF file")?;

    let mut data = &buffer[..];

    // Aggregation State
    let mut stats_map: HashMap<i32, FunctionStats> = HashMap::new();
    let mut stack: Vec<StackFrame> = Vec::new();

    let _event_count = 0;

    while data.has_remaining() {
        let len = prost::decode_length_delimiter(&mut data)
            .context("Failed to decode length delimiter")?;

        if data.remaining() < len {
            anyhow::bail!("Unexpected EOF");
        }
        let msg_slice = &data[..len];
        let event = ProfileEvent::decode(msg_slice).context("Failed to decode ProfileEvent")?;
        data.advance(len);

        process_event_for_stats(&event, &mut stack, &mut stats_map, pc_map);
    }

    let empty_source_map = HashMap::new();
    let source_map = debug_info
        .as_ref()
        .map(|d| &d.pc_to_source)
        .unwrap_or(&empty_source_map);

    print_report(&stats_map, source_map, args.show_callstacks);

    Ok(())
}

fn process_event_for_stats(
    event: &ProfileEvent,
    stack: &mut Vec<StackFrame>,
    stats: &mut HashMap<i32, FunctionStats>,
    pc_map: &HashMap<i32, String>,
) {
    let timestamp = event.timestamp.unwrap_or(0);
    let event_type = event.event_type.unwrap_or(0);
    let pc = event.pc.unwrap_or(0);

    // Use raw integer matching now for robustness against Type 1 (Native Enter)

    match event_type {
        1 | 2 => {
            // ENTER
            stack.push(StackFrame {
                pc,
                start_time: timestamp,
                children_time: 0,
            });
        }
        3 => {
            // EXIT
            if let Some(frame) = stack.pop() {
                // Determine duration
                let duration = timestamp - frame.start_time;
                let actual = duration - frame.children_time;

                // Update stats for THIS function
                let entry = stats.entry(frame.pc).or_insert_with(|| {
                    if let Some(name) = pc_map.get(&frame.pc) {
                        FunctionStats {
                            name: name.clone(),
                            call_count: 0,
                            total_time_us: 0,
                            actual_time_us: 0,
                            call_stacks: HashMap::new(),
                        }
                    } else {
                        // Heuristic for Unknown IDs
                        let name = if frame.pc >= 0x40000000 {
                            format!("<Native Code> ({})", frame.pc & 0x0FFFFFFF)
                        } else if frame.pc >= 0x30000000 {
                            format!("<API Code> ({:08x})", frame.pc)
                        } else if frame.pc >= 0x10000000 {
                            format!("<App Code> ({:08x})", frame.pc)
                        } else {
                            format!("Unknown_{}", frame.pc)
                        };

                        FunctionStats {
                            name,
                            call_count: 0,
                            total_time_us: 0,
                            actual_time_us: 0,
                            call_stacks: HashMap::new(),
                        }
                    }
                });
                entry.call_count += 1;
                entry.total_time_us += duration as u64;
                entry.actual_time_us += if actual < 0 { 0 } else { actual as u64 }; // Clamp

                // Track Call Stack (Parents only)
                let trace: Vec<i32> = stack.iter().map(|f| f.pc).collect();
                *entry.call_stacks.entry(trace).or_insert(0) += 1;

                // Add to parent children time
                if let Some(parent) = stack.last_mut() {
                    parent.children_time += duration;
                }
            }
        }
        _ => {}
    }
}

fn print_report(
    stats: &HashMap<i32, FunctionStats>,
    source_map: &HashMap<i32, SourceLocation>,
    show_callstacks: bool,
) {
    // Format: Function | Total Time (us) | Actual Time (us) | Average Time (us) | Call Count
    println!(
        "{:<60} | {:>15} | {:>16} | {:>18} | {:>10}",
        "Function", "Total Time (us)", "Actual Time (us)", "Average Time (us)", "Call Count"
    );
    println!("{:-<133}", ""); // Separator

    // Sort by Total Time desc
    let mut sorted_stats: Vec<&FunctionStats> = stats.values().collect();
    sorted_stats.sort_by(|a, b| b.total_time_us.cmp(&a.total_time_us));

    for s in sorted_stats {
        let avg = if s.call_count > 0 {
            s.total_time_us as f64 / s.call_count as f64
        } else {
            0.0
        };
        println!(
            "{:<60} | {:>15} | {:>16} | {:>18.3} | {:>10}",
            s.name, s.total_time_us, s.actual_time_us, avg, s.call_count
        );

        if show_callstacks && !s.call_stacks.is_empty() {
            println!("    Call Stacks:");
            let mut sorted_stacks: Vec<(&Vec<i32>, &u64)> = s.call_stacks.iter().collect();
            // Sort by count desc
            sorted_stacks.sort_by(|a, b| b.1.cmp(a.1));

            for (stack, count) in sorted_stacks {
                println!("      [{}] Calls:", count);
                if stack.is_empty() {
                    println!("        <Native Code>");
                }
                for &pc in stack.iter().rev() {
                    // Resolve PC
                    let mut name = format!("Unknown_{}", pc);
                    let mut file = "".to_string();
                    let mut line = "".to_string();

                    if let Some(src) = source_map.get(&pc) {
                        name = src.symbol.clone();
                        file = src.file.clone();
                        line = src.line.to_string();
                    } else {
                        // Fallback naming
                        if pc >= 0x40000000 {
                            name = format!("<Native Code>"); // Typical label in stack trace
                        } else if pc >= 0x30000000 {
                            name = format!("<API Code>");
                        } else if pc >= 0x10000000 {
                            name = format!("<App Code>");
                        }
                    }

                    if !file.is_empty() {
                        println!("        {:<40} {:<60} {:<5}", name, file, line);
                    } else {
                        println!("        {:<40}", name);
                    }
                }
                println!("");
            }
        }
    }
}

fn parse_debug_xml(path: &str) -> Result<DebugInfo> {
    let file = File::open(path).context("Failed to open debug XML")?;
    let file = BufReader::new(file);
    let parser = EventReader::new(file);

    let mut pc_to_name = HashMap::new();
    let mut pc_to_source = HashMap::new();

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "functionEntry" {
                    let mut func_name = String::new();
                    let mut parent_name = String::new();
                    let mut start_pc = -1;

                    for attr in attributes {
                        match attr.name.local_name.as_str() {
                            "name" => func_name = attr.value,
                            "parent" => parent_name = attr.value,
                            "startPc" => start_pc = attr.value.parse().unwrap_or(-1),
                            _ => {}
                        }
                    }

                    if start_pc != -1 {
                        let full_name = if !parent_name.is_empty() {
                            format!("{}.{}", parent_name, func_name)
                        } else {
                            func_name
                        };
                        pc_to_name.insert(start_pc, full_name);
                    }
                } else if name.local_name == "entry" {
                    // Parse pcToLineNum entries
                    // <entry filename="..." id="1" lineNum="12" parent="globals/SampleView" pc="268435460" symbol="<init>"/>
                    let mut filename = String::new();
                    let mut line_num = -1;
                    let mut pc = -1;
                    let mut symbol = String::new();
                    let mut parent = String::new();

                    for attr in attributes {
                        match attr.name.local_name.as_str() {
                            "filename" => filename = attr.value,
                            "lineNum" => line_num = attr.value.parse().unwrap_or(-1),
                            "pc" => pc = attr.value.parse().unwrap_or(-1),
                            "symbol" => symbol = attr.value,
                            "parent" => parent = attr.value,
                            _ => {}
                        }
                    }

                    if pc != -1 {
                        // Clean up parent (e.g. "globals/SampleView" -> "SampleView")
                        let clean_parent = parent.replace("globals/", "");
                        let full_name = if !clean_parent.is_empty() {
                            format!("{}.{}", clean_parent, symbol)
                        } else {
                            symbol.clone()
                        };

                        pc_to_source.insert(
                            pc,
                            SourceLocation {
                                file: filename,
                                line: line_num,
                                symbol: full_name,
                            },
                        );
                    }
                }
            }
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
    }

    Ok(DebugInfo {
        pc_to_name,
        pc_to_source,
    })
}
