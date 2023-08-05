//！ Store some entity.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MDAIndex {
    pub header_offset: u64,
    pub train_data_offset: u64,
    pub anno_entries_offset: u64,
    pub anno_headers_offset: u64,
}

/// Define the MDAHeader structure
#[derive(Serialize, Deserialize, Debug)]
pub struct MDAHeader {
    pub tags: Vec<String>,
    pub train_data: TrainData,
}

/// Define the train_data_index in header
#[derive(Serialize, Deserialize, Debug)]
pub struct TrainData {
    pub data_type: String,
    pub metadata: String,
}

/// Type of training data
#[derive(Serialize, Deserialize, Debug)]
pub enum TrainingData {
    Text(String),
    Image(Vec<u8>),
    Video(Vec<u8>),
    Audio(Vec<u8>),
}

/// Type of training data
#[derive(Serialize, Deserialize, Debug)]
pub enum DataType {
    Text,
    Image,
    Video,
    Audio,
}

/// Used to store the image metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct ImageMetaData {
    pub size: (u32, u32),
    pub channel_count: u8,
    pub color_space: String,
}

/// Used to store the text metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct TextMetaData {
    pub length: usize,
    pub encoding: String,
    pub vocabulary_size: usize,
}

/// Used to store the aduio metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct AudioMetaData {
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

// VideoMetaData
#[derive(Debug, Clone)]
pub struct VideoMetaData {
    pub duration: f64,
    pub resolution: (u16, u16),
}
