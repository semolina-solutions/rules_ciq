//! Extracts the application ID from a ConnectIQ manifest file.
//!
//! Usage:
//!   get_application_id <manifest_xml_path>
//!
//! This tool parses the provided `manifest.xml` file and prints the `id` attribute
//! of the `iq:application` element to stdout.

use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;
use xml::reader::{EventReader, XmlEvent};

#[derive(Parser)]
struct Args {
    manifest_xml_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let content = match fs::read_to_string(&args.manifest_xml_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "Error reading file '{}': {}",
                args.manifest_xml_path.display(),
                e
            );
            process::exit(1);
        }
    };

    match extract_app_id(&content) {
        Ok(id) => println!("{}", id),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn extract_app_id(xml: &str) -> Result<String, String> {
    let parser = EventReader::from_str(xml);

    for event in parser {
        match event {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "application" {
                    for attr in attributes {
                        if attr.name.local_name == "id" {
                            return Ok(attr.value);
                        }
                    }
                }
            }
            Err(e) => return Err(format!("Error parsing XML: {}", e)),
            _ => {}
        }
    }

    Err("Could not find iq:application id attribute in XML".to_string())
}
