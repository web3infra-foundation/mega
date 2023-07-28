use anyhow::Result;
use bincode::serialize_into;
use chrono::Local;
use clap::Parser;
use mda::utils::*;
use mda::{combine_files, entity::*, get_files_in_folder};
use rayon::prelude::*;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;
 
/// Write data to an MDA file.
fn write_data_to_mda(
    file_path: &str,
    header: Header,
    train_data: TrainingData,
    anno_data: String,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file_path)?;

    let index_placeholder_offset = file.stream_position()?;

    serialize_into(
        &file,
        &Index {
            header_offset: 0,
            train_data_offset: 0,
            anno_data_offset: 0,
        },
    )?;

    let header_offset = file.stream_position()?;
    serialize_into(&file, &header)?;

    let train_data_offset = file.stream_position()?;
    match train_data {
        TrainingData::Text(t) => {
            serialize_into(&file, &DataType::Text)?;
            serialize_into(&file, &t)?
        }
        TrainingData::Image(i) => {
            serialize_into(&file, &DataType::Image)?;
            serialize_into(&file, &i)?
        }
        TrainingData::Video(v) => {
            serialize_into(&file, &DataType::Video)?;
            serialize_into(&file, &v)?
        }
        TrainingData::Audio(a) => {
            serialize_into(&file, &DataType::Audio)?;
            serialize_into(&file, &a)?
        }
    };

    let anno_data_offset = file.stream_position()?;
    file.write_all(anno_data.as_bytes())?;

    file.seek(SeekFrom::Start(index_placeholder_offset))?;
    serialize_into(
        &file,
        &Index {
            header_offset,
            train_data_offset,
            anno_data_offset,
        },
    )?;

    Ok(())
}

/// Read data from an MDA file. 
fn read_data_from_mda(file_path: &str) -> Result<(Index, Header), Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let index: Index = bincode::deserialize_from(&mut reader)?;
    

    reader.seek(SeekFrom::Start(index.header_offset))?;
    let header:Header = bincode::deserialize_from(&mut reader)?;
    Ok((index, header))
}

/// Extract data from an MDA file.
fn extract_data_from_mda(
    mda_path: &str,
    training_data_path: &str,
    anno_data_path: &str,
) -> Result<(TrainingData, Vec<String>), Box<dyn Error>> {
    let file = File::open(mda_path)?;
    let mut reader = BufReader::new(file);
    let index: Index = bincode::deserialize_from(&mut reader)?;
    let train_data: TrainingData;
    let mut anno_data: Vec<String> = Vec::new();

    reader.seek(SeekFrom::Start(index.train_data_offset))?;
    let data_type: DataType = bincode::deserialize_from(&mut reader)?;
    match data_type {
        DataType::Text => {
            let text: String = bincode::deserialize_from(&mut reader)?;
            train_data = TrainingData::Text(text.clone());

            save_text_to_file(&text, training_data_path)?;
        }
        DataType::Image => {
            let image_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            train_data = TrainingData::Image(image_data.clone());

            save_image_to_file(&image_data, training_data_path)?;
        }
        DataType::Video => {
            let video_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            train_data = TrainingData::Video(video_data.clone());

            save_video_to_file(&video_data, training_data_path)?;
        }
        DataType::Audio => {
            let audio_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            train_data = TrainingData::Audio(audio_data.clone());

            save_audio_to_file(&audio_data, training_data_path)?;
        }
    };

    reader.seek(SeekFrom::Start(index.anno_data_offset))?;
    let mut line = String::new();
    while let Ok(bytes_read) = reader.read_line(&mut line) {
        if bytes_read == 0 {
            break;
        }
        anno_data.push(line.trim().to_string());
        line.clear();
    }
    write_strings_to_file(&anno_data, anno_data_path)?;
    println!("Extract data {:?} successfully",mda_path);
    Ok((train_data, anno_data))
}

/// Extract metadata from training data 
//TODO
fn process_file(file_path: &str) -> Option<Box<dyn std::any::Any>> {
    if file_path.ends_with(".jpg") || file_path.ends_with(".png") {
        let image_metadata = extract_image_metadata(file_path);
        Some(Box::new(image_metadata) as Box<dyn std::any::Any>)
    } else if file_path.ends_with(".mp4") || file_path.ends_with(".avi") {
        // TODO
        None
    } else if file_path.ends_with(".mp3") || file_path.ends_with(".wav") {
        match extract_audio_metadata(file_path) {
            Ok(audio_metadata) => return Some(Box::new(audio_metadata) as Box<dyn std::any::Any>),
            Err(err) => eprintln!("Error: {}", err),
        }
        None
    } else if file_path.ends_with(".txt") || file_path.ends_with(".docx") {
        let text_metadata = extract_text_metadata(file_path);
        Some(Box::new(text_metadata) as Box<dyn std::any::Any>)
    } else {
        None
    }
}

/// Generate MDA file
fn generate_mda_file(
    training_data: &str,
    annotation_data: &str,
    output: &str,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let filename = extract_filename_change_extension(training_data);
    let output_path = output.to_owned() + filename;

    let metadata = match process_file(training_data) {
        Some(metadata) => metadata,
        None => {
            println!("Failed to extract metadata!");
            std::process::exit(0);
        }
    };
   
    let  meta:String;

    if let Some(image_metadata) = metadata.downcast_ref::<ImageMetaData>() {
        meta = format!("Image metadata: {:?}", image_metadata);
    } else if let Some(text_metadata) = metadata.downcast_ref::<TextMetaData>() {
        meta = format!("Text metadata: {:?}", text_metadata);
    } else {
        println!("Unknown metadata type");
        std::process::exit(0);
    }

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

    let header = Header {
        tags,
        train_data: TrainData {
            data_type: file_type,
            metadata: meta,
        },
    };

    let train_data = match config_training_data(training_data) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("Fail to load training data {}", error);
            std::process::exit(0);
        }
    };

    let anno_data = match config_annotation(annotation_data) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Fail to load annotation data {}", err);
            std::process::exit(0);
        }
    };
   
    write_data_to_mda(&output_path, header, train_data, anno_data)?;
    Ok(())
}

pub fn main() {
    let args = Config::parse();

    match run(args) {
        Ok(results) => results,
        Err(err) => {
            eprintln!("Application: {}", err);
            std::process::exit(0);
        }
    };
}


pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    // Generate .mda file
    if config.action == "generate" {
        let start_time = Instant::now();
        println!(
            "\x1b[38;5;208m[WARN]\x1b[0m[{}] Start to generate mda files...",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
        );
        match (&config.train, &config.anno, &config.output) {
            (Some(train_data), Some(anno_data), Some(output)) => {
                if is_directory(train_data) && is_directory(anno_data) && is_directory(output) {
                    let train_files = get_files_in_folder(train_data);
                    let anno_files = get_files_in_folder(anno_data);
                    let file_combinations = combine_files(train_files, anno_files);
                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(config.threads.unwrap_or(3)) // 设置线程池的线程数量
                        .build()
                        .unwrap();

                    pool.install(|| {
                        file_combinations
                            .par_iter()
                            .for_each(|(train_file, anno_file)| {
                                generate_mda_file(
                                    train_file.to_str().unwrap(),
                                    anno_file.to_str().unwrap(),
                                    output,
                                    &config,
                                )
                                .unwrap_or_else(|err| {
                                    eprintln!("Failed to process file combination: {}", err);
                                });
                            });
                    });
                    let end_time = Instant::now();
                    let duration = end_time - start_time;
                    println!(
                        "\x1b[38;5;208m[WARN]\x1b[0m[{}] {} mda files have been generated in {:?}",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        file_combinations.len(),
                        duration
                    );
                } else if is_file(train_data) && is_file(anno_data) {
                    generate_mda_file(train_data, anno_data, output, &config)?;
                    let end_time = Instant::now();
                    let duration = end_time - start_time;
                    println!(
                        "\x1b[38;5;208m[WARN]\x1b[0m[{}] 1 mda files have been generated in {:?}",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        duration
                    );
                } else {
                    println!("Please input the correct path for training data, annotation data and output");
                }
            }
            _ => {
                println!("Please input training data, annotation data and output");
                std::process::exit(0);
            }
        };
    } else if config.action == "list" {
        match &config.mda {
            Some(mda) => {
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    for file in mda_files {
                        match read_data_from_mda(&file) {
                            Ok((index, header)) => {
                               
                                println!("file: {}", file);
                                println!("{:?}", format!("{}",index));
                                println!("{:#?}", format!("{}",header));
                                println!("--------------------------");

                            }
                            Err(err) => {
                                eprintln!("Failed to read data from MDA file: {}", err);
                            }
                        }
                    }
                } else if is_file(mda) {
                    match read_data_from_mda(mda) {
                        Ok((index, header)) => {
                            println!("{:#?}", index);
                            println!("{:#?}", header);
                        }
                        Err(err) => {
                            eprintln!("Failed to read data from MDA file: {}", err);
                        }
                    }
                } else {
                }
            }
            _ => {
                println!("Please input mda file");
                std::process::exit(0);
            }
        }
        // TODO: version control
    } else if config.action == "update" {
    } else if config.action == "extract" {
        match (&config.train, &config.anno, &config.mda) {
            (Some(train_data), Some(anno_data), Some(mda)) => {
                if is_directory(mda)   {
                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    for file in mda_files {
                        let train_data = train_data.to_string() + &extract_file_name(&file);
                        let anno_data: String = anno_data.to_string() + &extract_file_name(&file);
                        let _ =extract_data_from_mda(&file, &train_data, &anno_data);
                    }
                } else if is_file(mda)  {
                    let train_data = train_data.to_string() + &extract_file_name(mda);
                    let anno_data: String = anno_data.to_string() + &extract_file_name(mda);
                    let _ =extract_data_from_mda(mda, &train_data, &anno_data);

                } else {
                    println!("Please input training data, annotation data and output");
                }
            }
            _ => {
                println!("Please input training data, annotation data and output");
                std::process::exit(0);
            }
        }
    } else {
        println!("wrong action");
    }
    Ok(())
}
