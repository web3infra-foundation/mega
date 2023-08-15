//! Used to map the traning data and its annotation data
//! Case1: All the annotation data is stored in one CSV or JSON pr TXT file and it needs to be parsed
//!
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Matching training data and annotation data
pub fn get_train_path_and_anno_content(file_path: &str, start_line: usize,end_line: usize) -> Vec<AnnoInfo> {
 
    if file_path.ends_with("txt") {
        read_txt_file_info(file_path, start_line,end_line)
    } else if file_path.ends_with("csv") {
        read_csv_file_info(file_path, start_line,end_line).unwrap()
    } else if file_path.ends_with("json") {
        read_json_file_info(file_path, start_line,end_line).unwrap()
    } else {
        std::process::exit(0);
    }
}
/// Record the training data name and annotation content
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnnoInfo {
    /// training data name 
    pub file_name: String,
    ///  annotation content
    pub content: String,
}
impl AnnoInfo {
    fn from_json_object(json_object: &Value) -> Option<Self> {
        let file_name = json_object["filename"].as_str()?.to_string();
        let content = serde_json::to_string_pretty(json_object).ok()?;
        Some(Self { file_name, content })
    }
    pub fn new()->AnnoInfo{
        AnnoInfo { file_name: "".to_string(), content: "".to_string() }
    }
}
fn read_txt_file_info(file_path: &str, start_line: usize, end_line: usize) -> Vec<AnnoInfo> {
    // 省略文件打开和错误处理

    let file = File::open(file_path).expect("Failed to open the file");
    let reader = BufReader::new(file);
    let mut txt_info_vec: Vec<AnnoInfo> = Vec::new();
    let mut current_line = 1;

    for (_line_number, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");

        if current_line < start_line {
            current_line += 1;
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let txt_info = AnnoInfo {
                file_name: parts[0].to_string(),
                content: parts[1].to_string(),
            };
            txt_info_vec.push(txt_info);
        }

        current_line += 1;

        if end_line != 0 && current_line > end_line {
            break;
        }
    }

    txt_info_vec
}

fn read_csv_file_info(file_path: &str, start_line: usize, end_line: usize) -> Result<Vec<AnnoInfo>, Box<dyn Error>> {
    // 省略文件打开和错误处理

    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().from_reader(file);
    let mut csv_info_vec: Vec<AnnoInfo> = Vec::new();
    let mut current_line = 1;

    for (_line_number, result) in rdr.records().enumerate() {
        let record = result.expect("Failed to read record");

        if current_line < start_line {
            current_line += 1;
            continue;
        }

        if record.len() >= 2 {
            let file_name = record[0].to_string();
            let content = record.iter().skip(1).collect::<Vec<_>>().join(" ");
            let csv_info = AnnoInfo { file_name, content };
            csv_info_vec.push(csv_info);
        }

        current_line += 1;

        if end_line != 0 && current_line > end_line {
            break;
        }
    }

    Ok(csv_info_vec)
}

fn read_json_file_info(file_path: &str,  start_line: usize,end_line: usize) -> Result<Vec<AnnoInfo>, Box<dyn Error>> {
    // 省略文件打开和错误处理

    let file = File::open(file_path)?;
    let json_data: Value = serde_json::from_reader(file)?;
    let mut json_info_vec: Vec<AnnoInfo> = Vec::new();
    let mut current_line = 1;

    for (_key, value) in json_data.as_object().unwrap() {
        if current_line < start_line {
            current_line += 1;
            continue;
        }

        if let Some(json_info) = AnnoInfo::from_json_object(value) {
            json_info_vec.push(json_info);
        }

        current_line += 1;

        if end_line != 0 && current_line > end_line {
            break;
        }
    }

    Ok(json_info_vec)
}

 
