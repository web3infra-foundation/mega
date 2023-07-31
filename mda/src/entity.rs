//！ Store some common entity.
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Command Line Tool
#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "", long_about = "", after_help = "")]
#[derive(Deserialize, Serialize)]
pub struct Config {
    /// 4 actions: generate, extract, list,
    #[arg(long)]
    pub action: String,

    /// The path to train data
    #[arg(long)]
    pub train: Option<String>,

    /// The path to annotation data
    #[arg(long)]
    pub anno: Option<String>,

    /// The path output file
    #[arg(long)]
    pub output: Option<String>,

    /// The path to .mda file
    #[arg(long)]
    pub mda: Option<String>,

    /// The special version
    #[arg(long)]
    pub tags: Option<String>,

    /// Maximum number of threads  
    #[arg(long, default_value = "10")]
    pub threads: Option<usize>,

    /// Maximum number of threads  
    #[arg(long, default_value = "-1")]
    pub rev: Option<i32>,

    /// The special version
    #[arg(long)]
    pub add_tags: Option<String>,
}
 

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
impl fmt::Display for MDAHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MDAHeader {{ tags: [")?;

        for (i, tag) in self.tags.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", tag)?;
        }

        write!(f, "], train_data: {} }}", self.train_data)
    }
}

impl fmt::Display for TrainData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, " {{  metadata: {} }}", self.metadata)
    }
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
pub struct AudioMetaData {
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

// VideoMetaData( TODO )
