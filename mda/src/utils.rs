//! It includes some common functionalities, helper functions,
//! that help simplify the development process and provide shared functionalities.

extern crate image;
use crate::{
    AnnoOffset, AudioMetaData, ImageMetaData, MDAHeader , MDAIndex, TextMetaData,
    VideoMetaData,
};
use anyhow::Context;
use chrono::Local;
use encoding::{DecoderTrap, Encoding};
use hound::{Error as boundError, WavReader};
use image::{ColorType, GenericImageView};
use mp4parse::read_mp4;
use prettytable::{Cell, Row, Table};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use walkdir::WalkDir;

/// Information prompts
pub mod message {
    pub const GENERATE_MSG: &str = "Fail to generate mda files!";
    pub const INVALID_PATH_MSG: &str =
        "Please input the correct path for training data, annotation data and output data!";
    pub const FAIL_TO_READ: &str = "Failed to read data from MDA file!";
}

/// Get the file name of the input path
pub fn extract_file_name(file_path: &str) -> String {
    let path = Path::new(file_path);
    let file_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    file_name.to_string()
}

/// Get the file name and assign .mda extension
pub fn extract_filename_change_extension(path: &str) -> &str {
    let filename = path.rsplit('/').next().unwrap_or("");
    let new_filename = format!(
        "{}.mda",
        &filename[..filename.rfind('.').unwrap_or(filename.len())]
    );
    Box::leak(new_filename.into_boxed_str())
}

/// Save text file
pub fn save_text_to_file(text: &str, file_path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = file_path.to_string() + ".txt";
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(text.as_bytes())?;
    Ok(())
}
/// Save image file
pub fn save_image_to_file(image_data: &[u8], file_path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = file_path.to_string() + ".png";
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(image_data)?;
    Ok(())
}
/// Save video file
pub fn save_video_to_file(video_data: &[u8], file_path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = file_path.to_string() + ".mp4";
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(video_data)?;
    Ok(())
}
/// Save aduio file
pub fn save_audio_to_file(audio_data: &[u8], file_path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = file_path.to_string() + ".wav";
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(audio_data)?;
    Ok(())
}

/// Extract metadata from training data(image)
pub fn extract_image_metadata(image_path: &str) -> ImageMetaData {
    let msg = "Failed to open file ".to_owned() + image_path.clone();
    let image = image::open(image_path).expect(&msg);

    let (width, height) = image.dimensions();
    let channel_count = match image.color() {
        ColorType::L8 => 1,
        ColorType::La8 => 2,
        ColorType::Rgb8 => 3,
        ColorType::Rgba8 => 4,
        _ => panic!("Unsupported color type"),
    };
    let color_space = match image {
        image::DynamicImage::ImageRgb8(_) => "RGB".to_string(),
        image::DynamicImage::ImageRgba8(_) => "RGBA".to_string(),
        _ => "Unknown".to_string(),
    };

    ImageMetaData {
        size: (width, height),
        channel_count,
        color_space,
    }
}

/// Extract metadata from training data(text)
pub fn extract_text_metadata(text_path: &str) -> TextMetaData {
    let text = fs::read_to_string(text_path).expect("Failed to read text file");

    let length = text.chars().count();

    let (decoded_text, encoding) = match text.starts_with('\u{FEFF}') {
        true => {
            let decoded = encoding::all::UTF_8
                .decode(text[3..].as_bytes(), DecoderTrap::Replace)
                .unwrap();
            (decoded, "UTF-8")
        }
        false => {
            let decoded = encoding::all::ISO_8859_1
                .decode(text.as_bytes(), DecoderTrap::Replace)
                .unwrap();
            (decoded, "ISO-8859-1")
        }
    };

    let vocabulary_size = decoded_text
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>()
        .len();

    TextMetaData {
        length,
        encoding: encoding.to_string(),
        vocabulary_size,
    }
}

/// Extract metadata from training data(video)
pub fn extract_video_info(file_path: &str) -> Option<VideoMetaData> {
    let mut file = File::open(file_path).ok()?;
    let context = read_mp4(&mut file).ok()?;

    let video_track = context
        .tracks
        .iter()
        .find(|track| track.track_type == mp4parse::TrackType::Video)?;
    let duration = video_track.duration?;

    let media_timescale = context.timescale?.0;
    let total_time = duration.0 / 10 + duration.1 as u64;
    let track_duration_seconds = total_time as f64 / media_timescale as f64;

    if let Some(mp4parse::SampleEntry::Video(video_sample_entry)) = video_track
        .stsd
        .as_ref()
        .and_then(|stsd| stsd.descriptions.get(0))
    {
        let resolution = (video_sample_entry.width, video_sample_entry.height);
        return Some(VideoMetaData {
            duration: track_duration_seconds,
            resolution,
        });
    }

    None
}

/// Extract metadata from training data(audio)
pub fn extract_audio_metadata(file_path: &str) -> Result<AudioMetaData, boundError> {
    let reader = WavReader::open(file_path)?;
    let duration = reader.duration() as f64 / reader.spec().sample_rate as f64;

    let sample_rate = reader.spec().sample_rate;
    let channels = reader.spec().channels;
    let bit_depth = reader.spec().bits_per_sample;

    let audio_properties = AudioMetaData {
        duration,
        sample_rate,
        channels,
        bit_depth,
    };

    Ok(audio_properties)
}

/// Get the type of the file.
pub fn get_file_type(file_path: &str) -> Option<String> {
    if file_path.ends_with(".jpg") || file_path.ends_with(".png") || file_path.ends_with(".jpeg") {
        Some("Image".to_string())
    } else if file_path.ends_with(".mp4") || file_path.ends_with(".avi") {
        Some("Video".to_string())
    } else if file_path.ends_with(".mp3") || file_path.ends_with(".wav") {
        Some("Audio".to_string())
    } else if file_path.ends_with(".txt") || file_path.ends_with(".docx") {
        Some("Text".to_string())
    } else {
        None
    }
}

/// Find the .mda files in the folder.
pub fn find_mda_files_in_dir(dir: &Path, mda_files: &mut Vec<String>) {
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    // Check if the file ends with ".mda"
                    if file_name_str.ends_with(".mda") {
                        if let Some(file_path_str) = path.to_str() {
                            mda_files.push(file_path_str.to_string());
                        }
                    }
                }
            }
        }
    }
}

/// Check if it is a folder.
pub fn is_directory(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir()
}

/// Check if it is a file.
pub fn is_file(path: &str) -> bool {
    let path = Path::new(path);
    path.is_file()
}

/// Write content to files
pub fn write_strings_to_file(
    strings: &[String],
    output_path: &str,
    format: &str,
) -> anyhow::Result<()> {
    let output_path = output_path.to_string() + "." + format;
    let mut file = File::create(output_path).context("Failed to create output file")?;

    for string in strings {
        file.write_all(string.as_bytes())
            .context("Failed to write to output file")?;
        file.write_all(b"\n")
            .context("Failed to write newline to output file")?;
    }

    Ok(())
}

/// Record the start time
pub fn record_start_time(action: &str) -> Instant {
    let start_time = Instant::now();
    println!(
        "\x1b[38;5;208m[WARN]\x1b[0m[{}] Start to {} mda files...",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        action
    );
    start_time
}

/// Record the end time
pub fn record_end_time(start_time: Instant, number_of_mda_files: usize, action: &str) {
    let end_time = Instant::now();
    let duration = end_time - start_time;
    println!(
        "\n\x1b[38;5;208m[WARN]\x1b[0m[{}] {} mda files have been {} in {:?}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        number_of_mda_files,
        action,
        duration
    );
}

pub fn print_table_header() -> Table{
    let mut table1 = Table::new();

    table1.add_row(Row::new(vec![
        Cell::new("MDA File"),
        Cell::new("MDA Header Offset"),
        Cell::new("Training Data Offset"),
        Cell::new("Tags"),
        Cell::new("Training MetaData"),
    ]));

   
    table1
}

pub fn print_table_cell(file:&str,mut table: Table, index: MDAIndex, header: MDAHeader) -> Table {
    table.add_row(Row::new(vec![
        Cell::new(file),
        Cell::new(&index.header_offset.to_string()),
        Cell::new(&index.train_data_offset.to_string()),
        Cell::new(header.tags.join(", ").as_str()),
        Cell::new(&header.train_data.metadata),
    ]));
    table
}

use serde::Deserialize;
use std::process;

#[derive(Debug, Deserialize)]
pub struct AnnoConfigItem {
    #[serde(default = "default_id")]
    pub id: String,
    pub path: String,
    #[serde(default = "default_start")]
    pub start: usize,
    #[serde(default = "default_end")]
    pub end: usize,
}

fn default_id() -> String {
    "NONE".to_string()
}

fn default_start() -> usize {
    1
}
fn default_end() -> usize {
    0
}
#[derive(Debug, Deserialize)]
pub struct AnnoConfig {
    pub annotation: Vec<AnnoConfigItem>,
}
fn extract_id_from_path(path: &str) -> String {
    let path = Path::new(path);
    path.file_stem().unwrap().to_string_lossy().into_owned()
}
fn parse_and_process_toml(toml_content: &str) -> Result<Vec<AnnoConfigItem>, toml::de::Error> {
    let parsed_toml: Result<AnnoConfig, toml::de::Error> = toml::from_str(toml_content);

    match parsed_toml {
        Ok(anno_config) => {
            let mut annos = anno_config.annotation;

            for item in &mut annos {
                if item.id == "NONE" {
                    item.id = extract_id_from_path(&item.path); // Call the default_id function to extract ID
                }
            }

            Ok(annos)
        }
        Err(err) => Err(err),
    }
}

fn read_toml_file(filename: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(filename)
}
pub fn get_anno_config(path: &str) -> AnnoConfig {
    match read_toml_file(path) {
        Ok(toml_content) => match parse_and_process_toml(&toml_content) {
            Ok(annos) => AnnoConfig { annotation: annos },
            Err(err) => {
                eprintln!("Error parsing and processing TOML: {}", err);
                process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("Error reading the file: {}", err);
            process::exit(1);
        }
    }
}
pub fn create_anno_offsets(anno_config: &AnnoConfig) -> Vec<AnnoOffset> {
    let mut anno_offsets = Vec::new();

    for item in &anno_config.annotation {
        let anno_offset = AnnoOffset {
            id: item.id.clone(),
            header_offset: 0,
            entries_offset: 0,
        };
        anno_offsets.push(anno_offset);
    }

    anno_offsets
}
