use anyhow::Result;
use chrono::Local;
use clap::Parser;
 
 
use mda::utils::*;
use mda::{combine_files, entity::*, get_files_in_folder};
use rayon::prelude::*;
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use mda::generate::generate_mda_file;
use mda::update::update_anno_in_mda;
use mda::extract::read_info_from_mda;
use mda::extract::extract_data_from_mda;
use mda::extract::read_anno_from_mda;
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
                        .num_threads(config.threads.unwrap_or(3))
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
                        match read_info_from_mda(&file) {
                            Ok((index, header)) => {
                                println!("file: {}", file);
                                println!("{:?}", format!("{:?}", index));
                                println!("{:#?}", format!("{}", header));
                                println!("--------------------------");
                            }
                            Err(err) => {
                                eprintln!("Failed to read data from MDA file: {}", err);
                            }
                        }
                    }
                } else if is_file(mda) {
                    match read_info_from_mda(mda) {
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
        match (&config.mda, &config.anno) {
            (Some(mda), Some(anno_data)) => {
                if is_directory(mda) {
                    // TODO
                   
                } else if is_file(mda) {
                    match update_anno_in_mda(mda, anno_data) {
                        Ok(_) => {
                            println!("Update {:?} successfully",mda);
                        }
                        Err(err) => {
                            eprintln!("Failed to read data from MDA file: {}", err);
                        }
                    }
                } else {
                }
            }
            _ => {
                println!("Please input training data, annotation data and output");
                std::process::exit(0);
            }
        }

      
    } else if config.action == "version" {
        match &config.mda {
            Some(mda) => {
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    for file in mda_files {
                        match read_info_from_mda(&file) {
                            Ok((index, header)) => {
                                println!("file: {}", file);
                                println!("{:#?}", format!("{:?}", index));
                                println!("{:#?}", format!("{}", header));
                                println!("--------------------------");
                            }
                            Err(err) => {
                                eprintln!("Failed to read data from MDA file: {}", err);
                            }
                        }
                    }
                } else if is_file(mda) {
                    match read_anno_from_mda(mda, -1) {
                        Ok(_) => {}
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
    } else if config.action == "extract" {
        match (&config.train, &config.anno, &config.mda) {
            (Some(train_data), Some(anno_data), Some(mda)) => {
                let anno_version:i32=config.rev.unwrap_or_default();
            
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    for file in mda_files {
                        let train_data = train_data.to_string() + &extract_file_name(&file);
                        let anno_data: String = anno_data.to_string() + &extract_file_name(&file);
                        let _ = extract_data_from_mda(&file, &train_data, &anno_data, anno_version);
                    }
                } else if is_file(mda) {
                    let train_data = train_data.to_string() + &extract_file_name(mda);
                    let anno_data: String = anno_data.to_string() + &extract_file_name(mda);
                    let _ = extract_data_from_mda(mda, &train_data, &anno_data, anno_version);
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
