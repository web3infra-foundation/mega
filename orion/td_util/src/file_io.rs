/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use anyhow::Context;

use crate::{json::BUFFER_SIZE, zstd::is_zstd};

pub fn file_writer(file_path: &Path) -> anyhow::Result<Box<dyn Write>> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .with_context(|| format!("Unable to open file `{}` for writing", file_path.display()))?;
    if is_zstd(file_path) {
        let encoder = zstd::Encoder::new(file, zstd::DEFAULT_COMPRESSION_LEVEL)?.auto_finish();
        Ok(Box::new(BufWriter::with_capacity(BUFFER_SIZE, encoder)))
    } else {
        Ok(Box::new(BufWriter::with_capacity(BUFFER_SIZE, file)))
    }
}

pub fn file_reader(file_path: &Path) -> anyhow::Result<Box<dyn BufRead + Send>> {
    let file = File::open(file_path)
        .with_context(|| format!("Unable to open file `{}` for reading", file_path.display()))?;

    if is_zstd(file_path) {
        let decoder = zstd::Decoder::new(file)?;
        Ok(Box::new(BufReader::with_capacity(BUFFER_SIZE, decoder)))
    } else {
        Ok(Box::new(BufReader::with_capacity(BUFFER_SIZE, file)))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SerializationFormat {
    Json,
    JsonLines,
    Bincode,
}

pub fn detect_format_from_path(path: &Path) -> anyhow::Result<SerializationFormat> {
    let mut check_path = path;

    // Handle .zst extension by checking the file stem
    if path.extension().and_then(|s| s.to_str()) == Some("zst") {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            check_path = Path::new(stem);
        }
    }

    match check_path.extension().and_then(|s| s.to_str()) {
        Some("json") => Ok(SerializationFormat::Json),
        Some("jsonl") => Ok(SerializationFormat::JsonLines),
        Some("bin") => Ok(SerializationFormat::Bincode),
        _ => Err(anyhow::anyhow!(
            "Unknown format for path: {}. Supported: .json, .jsonl, .bin (with optional .zst compression)",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    static DATA: &str = "Artifact data";

    #[test]
    pub fn test_write_success() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_artifact.json");

        file_writer(&out_path)
            .unwrap()
            .write_all(DATA.as_bytes())
            .unwrap();

        let written = fs::read_to_string(&out_path).unwrap();
        assert_eq!(written, DATA);
    }

    #[test]
    pub fn test_write_error() {
        assert!(file_writer(Path::new("/invalid/file/path")).is_err());
    }

    #[test]
    pub fn test_zstd_encoding() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_artifact.zst");

        file_writer(&out_path)
            .unwrap()
            .write_all(DATA.as_bytes())
            .unwrap();

        assert!(out_path.exists());
        let compressed_data = fs::read(&out_path).unwrap();
        assert!(!compressed_data.is_empty());

        let file = fs::File::open(&out_path).unwrap();
        let mut decoder = zstd::Decoder::new(file).unwrap();
        let mut decompressed = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, DATA);
    }

    #[test]
    pub fn test_read_success() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_artifact.json");

        // Write data first
        file_writer(&out_path)
            .unwrap()
            .write_all(DATA.as_bytes())
            .unwrap();

        // Read it back
        let mut reader = file_reader(&out_path).unwrap();
        let mut read_data = String::new();
        std::io::Read::read_to_string(&mut reader, &mut read_data).unwrap();
        assert_eq!(read_data, DATA);
    }

    #[test]
    pub fn test_read_error() {
        assert!(file_reader(Path::new("/invalid/file/path")).is_err());
    }

    #[test]
    pub fn test_zstd_decoding() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_artifact.zst");

        // Write compressed data
        file_writer(&out_path)
            .unwrap()
            .write_all(DATA.as_bytes())
            .unwrap();

        // Read compressed data back
        let mut reader = file_reader(&out_path).unwrap();
        let mut decompressed = String::new();
        std::io::Read::read_to_string(&mut reader, &mut decompressed).unwrap();
        assert_eq!(decompressed, DATA);
    }

    #[test]
    pub fn test_round_trip_write_read() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_roundtrip.json");

        // Write data
        let mut writer = file_writer(&out_path).unwrap();
        writer.write_all(DATA.as_bytes()).unwrap();
        drop(writer); // Ensure file is closed

        // Read data back
        let mut reader = file_reader(&out_path).unwrap();
        let mut read_data = String::new();
        std::io::Read::read_to_string(&mut reader, &mut read_data).unwrap();

        assert_eq!(read_data, DATA);
    }

    #[test]
    pub fn test_round_trip_write_read_compressed() {
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("test_roundtrip.zst");

        // Write compressed data
        let mut writer = file_writer(&out_path).unwrap();
        writer.write_all(DATA.as_bytes()).unwrap();
        drop(writer); // Ensure file is closed

        // Read compressed data back
        let mut reader = file_reader(&out_path).unwrap();
        let mut decompressed = String::new();
        std::io::Read::read_to_string(&mut reader, &mut decompressed).unwrap();

        assert_eq!(decompressed, DATA);
    }
}
