//! It includes some common functionalities, helper functions,
//! that help simplify the development process and provide shared functionalities.

extern crate image;
use crate::{AudioMetaData, ImageMetaData, TextMetaData};
 
use anyhow::Context;
use encoding::{DecoderTrap, Encoding};
use hound::{Error as boundError, WavReader};
use image::{ColorType, GenericImageView};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufWriter,Write};
use std::path::Path;
use walkdir::WalkDir;
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
    let file_path = file_path.to_string() + ".mp3";
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(audio_data)?;
    Ok(())
}
/// Extract metadata from training data(image)
pub fn extract_image_metadata(image_path: &str) -> ImageMetaData {
    let image = image::open(image_path).expect("Failed to open image");

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

pub fn write_strings_to_file(strings: &[String], output_path: &str) -> anyhow::Result<()> {
    let output_path = output_path.to_string() + ".txt";
    let mut file = File::create(output_path).context("Failed to create output file")?;

    for string in strings {
        file.write_all(string.as_bytes())
            .context("Failed to write to output file")?;
        file.write_all(b"\n")
            .context("Failed to write newline to output file")?;
    }

    Ok(())
}
