use crate::read_from_file::{get_train_path_and_anno_content, AnnoInfo};
use crate::read_from_folders::{combine_files, get_files_in_folder};
use crate::run_mda::MDAOptions;
use crate::{
    extract_audio_metadata, extract_filename_change_extension, extract_image_metadata,
    extract_text_metadata, extract_video_info, get_anno_config, get_file_type, message, AnnoOffset,
    AudioMetaData, DataType, ImageMetaData, MDAHeader, MDAIndex, RevAnno,
    TextMetaData, TrainData, TrainingData, VideoMetaData,
};
use anyhow::Result;
use bincode::serialize_into;
use indicatif::ProgressBar;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
// Return the files in the targeted directory
pub fn list_files_in_directory(directory: &str) -> Result<Vec<String>, std::io::Error> {
    let dir_path = Path::new(directory);
    let mut file_paths = Vec::new();

    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(file_name) = file_path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        file_paths.push(file_name_str.to_string());
                    }
                }
            }
        }
    }

    Ok(file_paths)
}
/// extract last part from a path
fn extract_last_folder_name(path: &str) -> Option<&str> {
    let path = std::path::Path::new(path);
    if let Some(last_component) = path.components().next_back() {
        if let Some(folder_name) = last_component.as_os_str().to_str() {
            return Some(folder_name);
        }
    }
    None
}
/// get the file extension of the file
fn get_first_file_extension(folder_path: &str) -> Result<Option<String>, std::io::Error> {
    let dir_path = Path::new(folder_path);
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path.extension() {
                    return Ok(Some(extension.to_str().unwrap().to_string()));
                }
            }
        }
    }

    Ok(None)
}
/// config the completed path
fn generate_final_path(train: &str, annos: &str, extension: &str) -> String {
    let train_path = Path::new(train);
    let annos_path = Path::new(annos);

    if let Some(file_name) = annos_path.file_stem() {
        let new_file_name = format!("{}.{}", file_name.to_string_lossy(), extension);
        let final_path = train_path.join(new_file_name);
        final_path.to_string_lossy().into_owned()
    } else {
        panic!("Unable to generate final path.");
    }
}
// Generate mda files, case: 1 to many in directory
pub fn generate_mda_separate_annotations_one_to_many_in_folder(
    training_data: &str,
    annotation_group: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<usize, Box<dyn Error>> {
    // get folders
    let folders: Vec<&str> = annotation_group.split(',').collect();
    
    // get extension
    let extension = match get_first_file_extension(training_data) {
        Ok(extension) => match extension {
            Some(data) => data,
            None => "NONE".to_string(),
        },
        Err(err) => {
            eprintln!("Error: {}", err);
            "NONE".to_string()
        }
    };

    // config train and anno group
    let mut train_map_anno: Vec<TrainMapAnno> = Vec::new();
    for folder in folders {
        
        let id=extract_last_folder_name(folder).unwrap_or("NONE");
        let mut anno_groups: Vec<AnnoInfo> = Vec::new();
        match list_files_in_directory(folder) {
            Ok(file_paths) => {
                if file_paths.is_empty() {
                    println!("No files found in the directory.");
                } else {
                    for file_path in file_paths {
                        let item = folder.to_owned() + &file_path;
                        // get content
                        let mut file: File = File::open(item.clone())?;
                        let mut anno_data = String::new();
                        file.read_to_string(&mut anno_data)?;

                        let file_name = generate_final_path(training_data, &file_path, &extension);
                        let anno_info = AnnoInfo {
                            file_name ,
                            content: anno_data,
                        };

                        anno_groups.push(anno_info);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }

        let anno_item = TrainMapAnno {
            id: id.to_string(),
            data: anno_groups,
        };

        train_map_anno.push(anno_item);
    }
    let mut anno_groups: Vec<Annotation> = Vec::new();
    for (_index, item) in train_map_anno.iter().enumerate() {
        let id = item.id.clone();
        let data = &item.data;

        for tmp in data {
            let anno_for_single = AnnoItem {
                id: id.clone(),
                content: tmp.clone().content,
            };
            let anno = Annotation {
                file_name: tmp.clone().file_name,
                groups: vec![anno_for_single],
            };
            anno_groups.push(anno);
        }
    }
    let anno_groups = merge_annos(anno_groups);
    
    let pb = ProgressBar::new(anno_groups.len() as u64);
    // generate
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads.unwrap_or(10))
        .build()
        .unwrap();

    pool.install(|| {
        anno_groups.par_iter().for_each(|item| {
            match config_mda_content(&item.file_name, item.groups.clone(), output, config) {
                Ok(_) => {
                    pb.inc(1);
                }
                Err(err) => {
                    println!("Fail to generate {:?} {:?}", item.file_name, err);
                }
            }
        });
    });

    pb.finish_with_message("done");
    Ok(anno_groups.len())
}

/// Generate mda files, case: 1 to many file
pub fn generate_mda_separate_annotations_one_to_many(
    training_data: &str,
    annotation_group: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
    let pb = ProgressBar::new(1);

    // get paths
    let paths: Vec<&str> = annotation_group.split(',').collect();
    let mut anno_groups: Vec<AnnoItem> = Vec::new();
    for path in paths {
        // 1. read file content
        let mut file: File = File::open(path)?;
        let mut anno_data = String::new();
        file.read_to_string(&mut anno_data)?;
        // 2. config
        let anno_item = AnnoItem {
            id: path.to_string(),
            content: anno_data,
        };
        anno_groups.push(anno_item);
    }
    config_mda_content(training_data, anno_groups, output, config)?;
    pb.inc(1);
    pb.finish_with_message("done");
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


/// Extract metadata from training data
pub fn process_file(file_path: &str) -> Option<Box<dyn std::any::Any>> {
    if file_path.ends_with(".jpg") || file_path.ends_with(".png") {
        let image_metadata = extract_image_metadata(file_path);
        Some(Box::new(image_metadata) as Box<dyn std::any::Any>)
    } else if file_path.ends_with(".mp4") || file_path.ends_with(".avi") {
        match extract_video_info(file_path) {
            Some(info) => Some(Box::new(info) as Box<dyn std::any::Any>),
            None => None,
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
/// Map training data and anno data
#[derive(Debug, Clone)]
pub struct TrainMapAnno {
    /// anno group
    pub id: String,
    /// anno content
    pub data: Vec<AnnoInfo>,
}
#[derive(Debug, Clone)]
/// Record anno group and content
pub struct AnnoItem {
    pub id: String,
    pub content: String,
}

/// Record all the anno group
#[derive(Debug, Clone)]
pub struct Annotation {
    pub file_name: String,
    pub groups: Vec<AnnoItem>,
}

/// Generate mda files for combine case
pub fn generate_mda_combined_annotations(
    training_data: &str,
    anno_config: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<usize, Box<dyn Error>> {
    // config the annotation info
    let anno_config = get_anno_config(anno_config);

    // map training data, anno data
    let mut train_map_anno: Vec<TrainMapAnno> = Vec::new();
    for item in &anno_config.annotation {
        let map = get_train_path_and_anno_content(&item.path, item.start, item.end);
        let temp = TrainMapAnno {
            id: item.id.clone(),
            data: map,
        };
        train_map_anno.push(temp);
    }

    // group the train and anno
    let mut anno_groups: Vec<Annotation> = Vec::new();
    for (_index, item) in train_map_anno.iter().enumerate() {
        let id = item.id.clone();
        let data = &item.data;

        for tmp in data {
            let anno_for_single = AnnoItem {
                id: id.clone(),
                content: tmp.clone().content,
            };
            let anno = Annotation {
                file_name: tmp.clone().file_name,
                groups: vec![anno_for_single],
            };
            anno_groups.push(anno);
        }
    }
    let mut anno_groups = merge_annos(anno_groups);

    // assign training data file name
    for item in &mut anno_groups {
        item.file_name = training_data.to_owned() + &item.file_name.to_string();
    }

  
    // use pool to generate mda files
    let pb = ProgressBar::new(anno_groups.len() as u64);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads.unwrap_or(10))
        .build()
        .unwrap();

    pool.install(|| {
        anno_groups.par_iter().for_each(|item| {
            match config_mda_content(&item.file_name, item.groups.clone(), output, config) {
                Ok(_) => {
                    pb.inc(1);
                }
                Err(err) => {
                    eprintln!("Fail to generate {:?}! Error: {:?}", item.file_name, err);
                }
            }
        });
    });

    pb.finish_with_message("done");
    Ok(anno_groups.len())
}

/// Generate MDA file by content
pub fn config_mda_content(
    training_data: &str,
    anno_groups: Vec<AnnoItem>,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
    // config outpath
    let filename = extract_filename_change_extension(training_data);
    let output_path = output.to_owned() + filename;

    // MDAOptions MDAHeader Begin
    // MDAOptions MDAHeader -- config metadata
    let metadata = process_file(training_data)
        .ok_or(training_data.clone().to_owned() + "Failed to extract metadata!")?;

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
    let mut rev_anno_ids: Vec<RevAnnoWithID> = Vec::new();

    for item in anno_groups {
        let rev_anno = match config_annotation_data_by_content(&item.content) {
            Ok(rev_anno) => rev_anno,
            Err(err) => {
                eprintln!("Fail to load annotation data {}", err);
                std::process::exit(0);
            }
        };
        let temp = RevAnnoWithID {
            id: item.id,
            rev_anno,
        };
        rev_anno_ids.push(temp);
    }

    //Write data into mda file
    write_mda_data(&output_path, header, train_data, &mut rev_anno_ids)?;
    Ok(())
}


/// Write data into mda files
pub fn write_mda_data(
    file_path: &str,
    header: MDAHeader,
    train_data: TrainingData,
    rev_anno_ids:  &mut [RevAnnoWithID],
) -> Result<(), Box<dyn Error>> {
    let rev_anno_ids_clone = rev_anno_ids.to_owned();
    // create file
    let mut file = File::create(file_path)?;

    // record the position of the MDAIndex
    let index_placeholder_offset = file.stream_position()?;
    let mut tmp_anno_offsets: Vec<AnnoOffset> = Vec::new();
    for item in rev_anno_ids_clone {
        let tmp = AnnoOffset::new(&item.id);
        tmp_anno_offsets.push(tmp);
    }
    // Write MDAIndex into mda
    serialize_into(
        &file,
        &MDAIndex {
            header_offset: 0,
            train_data_offset: 0,
            annotations_offset: tmp_anno_offsets,
        },
    )?;

    // Get the MDAHeader Info and write into mda
    let header_offset = file.stream_position()?;
    serialize_into(&file, &header)?;

    // Write training data
    let train_data_offset = file.stream_position()?;
    match train_data {
        TrainingData::Text(t) => {
            serialize_into(&file, &DataType::Text)?;
            serialize_into(&file, &t)?;
        }
        TrainingData::Image(i) => {
            serialize_into(&file, &DataType::Image)?;
            serialize_into(&file, &i)?;
        }
        TrainingData::Video(v) => {
            serialize_into(&file, &DataType::Video)?;
            serialize_into(&file, &v)?;
        }
        TrainingData::Audio(a) => {
            serialize_into(&file, &DataType::Audio)?;
            serialize_into(&file, &a)?;
        }
    };

    //Config Anno data
    let mut tmp_anno_offsets_for_annotations: Vec<AnnoOffset> = Vec::new();

    for rev_anno_id in rev_anno_ids.iter_mut() {
        let mut rev_anno = rev_anno_id.clone().rev_anno;

        let mut anno_entries_offset = file.stream_position()?;
        let store_anno_entries_offset = anno_entries_offset;
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

        let tmp = AnnoOffset {
            id: rev_anno_id.id.clone(),
            header_offset: anno_headers_offset,
            entries_offset: store_anno_entries_offset,
        };
        tmp_anno_offsets_for_annotations.push(tmp);
    }

    // Return to MDAIndex and update offset
    file.seek(SeekFrom::Start(index_placeholder_offset))?;

    serialize_into(
        &file,
        &MDAIndex {
            header_offset,
            train_data_offset,
            annotations_offset: tmp_anno_offsets_for_annotations,
        },
    )?;

    Ok(())
}

/// Merge the same group(used to map train and anno)
pub fn merge_annos(annos: Vec<Annotation>) -> Vec<Annotation> {
    let mut merged_annos_map: HashMap<String, Vec<AnnoItem>> = HashMap::new();

    for anno in &annos {
        let file_name = &anno.file_name;
        let anno_value = anno.groups.clone();

        if let Some(existing_annos) = merged_annos_map.get_mut(file_name) {
            existing_annos.extend(anno_value);
        } else {
            merged_annos_map.insert(file_name.clone(), anno_value);
        }
    }

    let mut merged_annos: Vec<Annotation> = Vec::new();

    for (file_name, annos) in merged_annos_map.iter() {
        merged_annos.push(Annotation {
            file_name: file_name.clone(),
            groups: annos.clone(),
        });
    }

    merged_annos
}

/// Used to group anno data
#[derive(Debug, Clone)]
pub struct RevAnnoWithID {
    pub id: String,
    pub rev_anno: RevAnno,
}

/// Generate mda files, case: 1 to 1 
pub fn generate_mda_separate_annotation_1_1(
    training_data: &str,
    annotation_data: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
   
    // get anno content
    let mut file = File::open(annotation_data)?;
    let mut anno_data = String::new();
    file.read_to_string(&mut anno_data)?;

    // config anno id
    let id = extract_second_last_part(annotation_data,'/');
    let id = id.unwrap_or("NONE".to_string());
    let anno_item = AnnoItem {
        id: id.to_string(),
        content: anno_data,
    };

    // config mda
    config_mda_content(training_data, vec![anno_item], output, config)?;
 
    Ok(())
}

/// Generate mda files, case: 1 to 1 file
pub fn generate_mda_separate_annotation_one_to_one(
    training_data: &str,
    annotation_data: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<(), Box<dyn Error>> {
    let pb = ProgressBar::new(1);
     generate_mda_separate_annotation_1_1(training_data, annotation_data, output, config)?;
    pb.inc(1);
    pb.finish_with_message("done");
    Ok(())
}

/// Generate mda files, case: 1 to 1 in directory
pub fn generate_mda_separate_annotation_one_to_one_in_folder(
    train_path: &str,
    anno_path: &str,
    output: &str,
    config: &MDAOptions,
) -> Result<usize, Box<dyn Error>> {
    // map train and anno
    let train_files = get_files_in_folder(train_path);
    let anno_files = get_files_in_folder(anno_path);
    let file_combinations = combine_files(train_files, anno_files);
    // set progress bar
    let pb = ProgressBar::new(file_combinations.len() as u64);

    // use thread pool to generate files
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads.unwrap_or(10))
        .build()
        .unwrap();

    pool.install(|| {
        file_combinations
            .par_iter()
            .for_each(
                |(train_file, anno_file)| match generate_mda_separate_annotation_1_1(
                    train_file.to_str().unwrap(),
                    anno_file.to_str().unwrap(),
                    output,
                    config,
                ) {
                    Ok(_) => {
                        pb.inc(1);
                    }
                    Err(err) => {
                        eprintln!(
                            "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                            train_file.to_str().unwrap(),
                            message::GENERATE_MSG,
                            err
                        );
                    }
                },
            );
    });
    pb.finish_with_message("done");
    Ok(file_combinations.len())
}

/// extract second last part from a path
fn extract_second_last_part(input: &str, delimiter: char) -> Option<String> {
    let parts: Vec<&str> = input.split(delimiter).collect();
    
    if parts.len() >= 2 {
        Some(parts[parts.len() - 2].to_string())
    } else {
        None
    }
}


