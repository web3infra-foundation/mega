use crate::{
    extract_audio_metadata, extract_filename_change_extension, extract_image_metadata,
    extract_text_metadata, get_file_type, DataType, ImageMetaData, MDAHeader, MDAIndex,
    RevAnno, TextMetaData, TrainData, TrainingData, extract_video_info, AudioMetaData, VideoMetaData,
};
use anyhow::Result;
use bincode::serialize_into;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use crate::run_mda::MDAOptions;
/// Read anno content and generate mda file
pub fn generate_mda(
    training_data: &str,
    annotation_data: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(annotation_data)?;
    let mut anno_data = String::new();
    file.read_to_string(&mut anno_data)?;
    generate_mda_by_content(training_data, &anno_data, output, config)?;
    Ok(())
}

/// Generate MDA file by content
pub fn generate_mda_by_content(
    training_data: &str,
    annotation_data: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
    // MDAOptions filename and path
    let filename = extract_filename_change_extension(training_data);
    let output_path = output.to_owned() + filename;

    // MDAOptions MDAHeader Begin
    // MDAOptions MDAHeader -- config metadata
     let metadata = process_file(training_data)
        .ok_or(training_data.clone().to_owned() + "Failed to extract metadata!" )?;

    let meta: String;
    if let Some(image_metadata) = metadata.downcast_ref::<ImageMetaData>() {
        meta = format!("{:?}", image_metadata);
    } else if let Some(text_metadata) = metadata.downcast_ref::<TextMetaData>() {
        meta = format!("{:?}", text_metadata);
    } else if let Some(audio_metadata) = metadata.downcast_ref::<AudioMetaData>() {
        meta = format!("{:?}", audio_metadata);
    } else if let Some(video_metadata) = metadata.downcast_ref::<VideoMetaData>() {
        meta = format!("{:?}", video_metadata);
    } else {
        return Err("Unknown metadata type".into());
    }
    // MDAOptions MDAHeader -- config tags
    let tags = match &config.tags {
        Some(tags) => tags.split(',').map(|s| s.trim().to_string()).collect(),
        None => vec![],
    };

    let file_type = match get_file_type(training_data) {
        Some(file_type) => file_type,
        None => {
            println!("Unknown file type");
            std::process::exit(0);
        }
    };

    let header = MDAHeader {
        tags,
        train_data: TrainData {
            data_type: file_type.to_string(),
            metadata: meta.to_string(),
        },
    };
    // MDAOptions MDAHeader finish

    // MDAOptions Training Data
    let train_data = match config_training_data(training_data) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("Fail to load training data {}", error);
            std::process::exit(0);
        }
    };
    // MDAOptions Annotation data

    let mut anno_data = match config_annotation_data_by_content(annotation_data) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            eprintln!("Fail to load annotation data {}", err);
            std::process::exit(0);
        }
    };
    // Write data into mda file
    write_data_to_mda(&output_path, header, train_data, &mut anno_data)?;
    Ok(())
}

/// Get training data
pub fn config_training_data(file_path: &str) -> Result<TrainingData, String> {
    let path = Path::new(file_path);
    let mut file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Error reading file: {}", e))?;

    let file_extension = path.extension().and_then(|ext| ext.to_str());
    match file_extension {
        Some("txt") => Ok(TrainingData::Text(
            String::from_utf8_lossy(&buffer).to_string(),
        )),
        Some("jpg") | Some("jpeg") | Some("png") => Ok(TrainingData::Image(buffer)),
        Some("mp4") | Some("avi") => Ok(TrainingData::Video(buffer)),
        Some("wav") | Some("mp3") => Ok(TrainingData::Audio(buffer)),
        _ => Err(String::from("Unsupported file type")),
    }
}

/// Get annotation data
pub fn config_annotation_data_by_content(content: &str) -> Result<RevAnno, Box<dyn Error>> {
    Ok(RevAnno::set_initial_element(content))
}

// Create mda file and write data
pub fn write_data_to_mda(
    file_path: &str,
    header: MDAHeader,
    train_data: TrainingData,
    rev_anno: &mut RevAnno,
) -> Result<(), Box<dyn Error>> {
    // Open the file and create a File object to write data to the specified file path
    let mut file = File::create(file_path)?;

    // Record the current file position as the offset for the index placeholder, which will be used later to update the index information in the file
    let index_placeholder_offset = file.stream_position()?;

    // Serialize and write an initial MDAIndex struct to the file. This struct contains offsets for the header, training data, annotation headers, and annotation entries, all initialized to 0.
    serialize_into(
        &file,
        &MDAIndex {
            header_offset: 0,
            train_data_offset: 0,
            anno_headers_offset: 0,
            anno_entries_offset: 0,
        },
    )?;

    // Get the file position before writing the header data, which represents the starting position of the header data in the file
    let header_offset = file.stream_position()?;

    // Serialize and write the header data to the file
    serialize_into(&file, &header)?;

    // Get the file position before writing the training data, which represents the starting position of the training data in the file
    let train_data_offset = file.stream_position()?;

    // Depending on the type of training data, choose the appropriate serialization and writing operations
    match train_data {
        TrainingData::Text(t) => {
            serialize_into(&file, &DataType::Text)?; // Write the data type identifier for text type
            serialize_into(&file, &t)?; // Write the text data
        }
        TrainingData::Image(i) => {
            serialize_into(&file, &DataType::Image)?; // Write the data type identifier for image type
            serialize_into(&file, &i)?; // Write the image data
        }
        TrainingData::Video(v) => {
            serialize_into(&file, &DataType::Video)?; // Write the data type identifier for video type
            serialize_into(&file, &v)?; // Write the video data
        }
        TrainingData::Audio(a) => {
            serialize_into(&file, &DataType::Audio)?; // Write the data type identifier for audio type
            serialize_into(&file, &a)?; // Write the audio data
        }
    };

    // Configure annotation
    // Record the current offset as the starting position of the entries and update the RevlogIndex
    let mut anno_entries_offset = file.stream_position()?;
    let store_anno_entries_offset = anno_entries_offset;

    // Write the entries and record their lengths
    let mut lengths: Vec<u64> = Vec::new();
    for entry in &rev_anno.entries {
        let entry_bytes = bincode::serialize(entry)?;
        file.write_all(&entry_bytes)?;
        lengths.push(entry_bytes.len() as u64);
    }

    // Record the current offset as the starting position of the headers and update the RevlogIndex
    let anno_headers_offset = file.stream_position()?;

    // Write the headers and update their offsets in the vector
    for (rev_anno_header, &length) in rev_anno.headers.iter_mut().zip(lengths.iter()) {
        rev_anno_header.offset = anno_entries_offset;
        rev_anno_header.length = length;
        let header_bytes = bincode::serialize(rev_anno_header)?;
        file.write_all(&header_bytes)?;
        anno_entries_offset += length;
    }

    file.seek(SeekFrom::Start(index_placeholder_offset))?;

    serialize_into(
        &file,
        &MDAIndex {
            header_offset,
            train_data_offset,
            anno_entries_offset: store_anno_entries_offset,
            anno_headers_offset,
        },
    )?;

    Ok(())
}

/// Extract metadata from training data
pub fn process_file(file_path: &str) -> Option<Box<dyn std::any::Any>> {
    if file_path.ends_with(".jpg") || file_path.ends_with(".png") {
        let image_metadata = extract_image_metadata(file_path);
        Some(Box::new(image_metadata) as Box<dyn std::any::Any>)
    } else if file_path.ends_with(".mp4") || file_path.ends_with(".avi") {
        match  extract_video_info(file_path) {
             Some(info)=>  Some(Box::new(info) as Box<dyn std::any::Any>),
             None=>None
        } 
    } else if file_path.ends_with(".mp3") || file_path.ends_with(".wav") {
        match extract_audio_metadata(file_path) {
            Ok(audio_metadata) => return Some(Box::new(audio_metadata) as Box<dyn std::any::Any>),
            Err(err) => {
                eprintln!("Error: {}", err);
                None
            }
        }
    } else if file_path.ends_with(".txt") || file_path.ends_with(".docx") {
        let text_metadata = extract_text_metadata(file_path);
        Some(Box::new(text_metadata) as Box<dyn std::any::Any>)
    } else {
        None
    }
}
