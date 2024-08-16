use clap::Subcommand;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use crate::utils::{path, util};
use crate::utils::path_ext::PathExt;

#[derive(Subcommand, Debug)]
pub enum LfsCmds {
    /// View or add LFS paths to Libra Attributes
    Track {
        pattern: Option<Vec<String>>,
    },
    /// Remove LFS paths from Libra Attributes
    Untrack {
        path: Vec<String>,
    },
}

pub async fn execute(cmd: LfsCmds) {
    match cmd {
        LfsCmds::Track { pattern } => { // TODO: deduplicate
            let attr_path = path::attributes().to_string_or_panic();
            match pattern {
                Some(pattern) => {
                    add_lfs_patterns(&attr_path, pattern).unwrap();
                }
                None => {
                    let lfs_patterns = extract_lfs_patterns(&attr_path).unwrap();
                    if !lfs_patterns.is_empty() {
                        println!("Listing tracked patterns");
                        for p in lfs_patterns {
                            println!("    {} ({})", p, util::ATTRIBUTES); // '\t' seems to be 8 spaces, :(
                        }
                    }
                }
            }
        }
        LfsCmds::Untrack { path } => {
            let attr_path = path::attributes().to_string_or_panic();
            untrack_lfs_patterns(&attr_path, path).unwrap();
        }
    }
}

fn add_lfs_patterns(file_path: &str, patterns: Vec<String>) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(file_path)?;

    if file.metadata()?.len() > 0 {
        file.seek(SeekFrom::End(-1))?;

        let mut last_byte = [0; 1];
        file.read_exact(&mut last_byte)?;

        // ensure the last byte is '\n'
        if last_byte[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }

    let lfs_patterns = extract_lfs_patterns(file_path)?;
    for pattern in patterns {
        if lfs_patterns.contains(&pattern) {
            continue;
        }
        println!("Tracking \"{}\"", pattern);
        let pattern = format!("{} filter=lfs diff=lfs merge=lfs -text\n", pattern.replace(" ", r"\ "));
        file.write_all(pattern.as_bytes())?;
    }

    Ok(())
}

fn untrack_lfs_patterns(file_path: &str, patterns: Vec<String>) -> io::Result<()> {
    if !Path::new(file_path).exists() {
        return Ok(());
    }
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let mut matched_pattern = None;
        // delete the specified lfs patterns
        for pattern in &patterns {
            let pattern = pattern.replace(" ", r"\ ");
            if line.trim_start().starts_with(&pattern) && line.contains("filter=lfs") {
                matched_pattern = Some(pattern);
                break;
            }
        }
        match matched_pattern {
            Some(pattern) => println!("Untracking \"{}\"", pattern),
            None => lines.push(line),
        }
    }

    // clear the file
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    for line in lines {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

fn extract_lfs_patterns(file_path: &str) -> io::Result<Vec<String>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // ' ' needs '\' before it to be escaped
    let re = Regex::new(r"^\s*(([^\s#\\]|\\ )+)").unwrap();

    let mut patterns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if !line.contains("filter=lfs") {
            continue;
        }
        if let Some(cap) = re.captures(&line) {
            if let Some(pattern) = cap.get(1) {
                let pattern = pattern.as_str().replace(r"\ ", " ");
                patterns.push(pattern);
            }
        }
    }

    Ok(patterns)
}