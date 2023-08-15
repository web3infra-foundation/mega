use anyhow::Result;

use crate::extract::{read_anno_from_mda_v1, read_anno_groups_from_mda, extract_data_from_mda_v1};
use crate::generate::generate_mda_v1;
use crate::read_from_file::get_train_path_and_anno_content;
use crate::read_from_folders::*;
use crate::*;
use crate::{
    extract::{
        extract_data_from_mda, extract_data_from_mda_and_anno_in_one_file, read_anno_from_mda,
        read_info_from_mda,
    },
    generate::{generate_mda, generate_mda_by_content},
    update::{update_anno_in_mda, update_anno_in_mda_by_content},
};
use clap::Parser;
use indicatif::ProgressBar;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
/// Command Line Tool
#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "", long_about = "", after_help = "")]
#[derive(Deserialize, Serialize)]
pub struct MDAOptions {
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

    /// Read from which line of the annotation file.
    #[arg(long, default_value = "1")]
    pub start: Option<usize>,

    /// Read from which line of the annotation file.
    #[arg(long, default_value = "1")]
    pub end: Option<usize>,

    /// The type of the annotation data, txt,json
    #[arg(long, default_value = "txt")]
    pub format: Option<String>,

    /// The type of the annotation data, txt,json
    #[arg(long, default_value = "mda")]
    pub anno_config: Option<String>,

    /// The type of the annotation data, txt,json
    #[arg(long, default_value = "NONE")]
    pub group: Option<String>,
}
#[allow(unused_assignments)]
pub fn run(config: MDAOptions) -> Result<(), Box<dyn Error>> {
    // Generate .mda file
    if config.action == "generate" {
        // Record start time
        let start_time = record_start_time(&config.action);
        // Generate mda files
        let number_of_mda_files = match (&config.train, &config.anno, &config.output) {
            (Some(train_data), Some(anno_data), Some(output)) => {
                if is_directory(train_data) && is_directory(anno_data) && is_directory(output) {
                    // 1. Scan the files in the training data folder and the annotation data folder.
                    // map train and anno
                    let train_files = get_files_in_folder(train_data);
                    let anno_files = get_files_in_folder(anno_data);
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
                            .for_each(|(train_file, anno_file)| {
                                match generate_mda(
                                    train_file.to_str().unwrap(),
                                    anno_file.to_str().unwrap(),
                                    output,
                                    &config,
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
                                }
                            });
                    });
                    pb.finish_with_message("done");
                    file_combinations.len()
                } else if is_file(train_data) && is_file(anno_data) {
                    // 2. Scan a specified individual file.
                    let pb = ProgressBar::new(1);
                    match generate_mda(train_data, anno_data, output, &config) {
                        Ok(_) => {
                            pb.inc(1);
                            pb.finish_with_message("done");
                            1
                        }
                        Err(err) => {
                            eprintln!(
                                "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                train_data,
                                message::GENERATE_MSG,
                                err
                            );
                            0
                        }
                    }
                } else if is_file(anno_data) && is_directory(train_data) {
                    // 3. Scan the training data folder and record the annotation data in a single file.
                    let mut train_anno_map = get_train_path_and_anno_content(
                        anno_data,
                        config.start.unwrap_or(0),
                        config.end.unwrap_or(0),
                    );
                    for item in &mut train_anno_map {
                        item.file_name = train_data.to_owned() + &item.file_name.to_string();
                    }
                    let pb = ProgressBar::new(train_anno_map.len() as u64);
                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(config.threads.unwrap_or(10))
                        .build()
                        .unwrap();

                    pool.install(|| {
                        train_anno_map.par_iter().for_each(|item| {
                            match generate_mda_by_content(
                                item.file_name.as_str(),
                                item.content.as_str(),
                                output,
                                &config,
                            ) {
                                Ok(_) => {
                                    pb.inc(1);
                                }
                                Err(err) => {
                                    eprintln!(
                                        "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                        item.file_name,
                                        message::GENERATE_MSG,
                                        err
                                    );
                                }
                            }
                        });
                    });
                    pb.finish_with_message("done");
                    train_anno_map.len()
                } else {
                    eprintln!("{}", message::INVALID_PATH_MSG);
                    0
                }
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        };
        // Record end time
        record_end_time(start_time, number_of_mda_files, "generated");
    } else if config.action == "list" {
        match &config.mda {
            Some(mda) => {
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();
                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);

                    let mut table = print_table_header();

                    for file in mda_files {
                        match read_info_from_mda(&file) {
                            Ok((index, header)) => {
                                table = print_table_cell(table.clone(), index, header);
                            }
                            Err(err) => {
                                eprintln!(
                                    "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                    mda,
                                    message::FAIL_TO_READ,
                                    err
                                );
                            }
                        }
                    }
                    table.printstd();
                } else if is_file(mda) {
                    match read_info_from_mda(mda) {
                        Ok((index, header)) => {
                            let table = print_table_header();
                            let table = print_table_cell(table, index, header);
                            table.printstd();
                        }
                        Err(err) => {
                            eprintln!(
                                "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                mda,
                                message::FAIL_TO_READ,
                                err
                            );
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

        // Record end time
        //    record_end_time(start_time, number_of_mda_files, "generated");
    } else if config.action == "update" {
        // Record start time
        let start_time = record_start_time(&config.action);

        let number_of_mda_files = match (&config.mda, &config.anno) {
            (Some(mda), Some(anno_data)) => {
                if is_directory(mda) && is_directory(anno_data) {
                    let mda_files = get_files_in_folder(mda);
                    let anno_files = get_files_in_folder(anno_data);
                    let file_combinations = combine_files(mda_files, anno_files);
                    let pb: ProgressBar = ProgressBar::new(file_combinations.len() as u64);

                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(config.threads.unwrap_or(10))
                        .build()
                        .unwrap();

                    pool.install(|| {
                        file_combinations
                            .par_iter()
                            .for_each(|(mda_file, anno_file)| {
                                update_anno_in_mda(
                                    mda_file.to_str().unwrap(),
                                    anno_file.to_str().unwrap(),
                                )
                                .unwrap_or_else(|err| {
                                    eprintln!("Failed to process file combination: {}", err);
                                });
                                pb.inc(1);
                            });
                    });
                    pb.finish_with_message("done");

                    file_combinations.len()
                } else if is_file(mda) && is_file(anno_data) {
                    match update_anno_in_mda(mda, anno_data) {
                        Ok(_) => {
                            let pb: ProgressBar = ProgressBar::new(1);

                            pb.inc(1);

                            pb.finish_with_message("done");
                        }
                        Err(err) => {
                            eprintln!("Failed to read data from MDA file: {}", err);
                        }
                    }
                    1
                } else if is_file(anno_data) && is_directory(mda) {
                    // 3. Scan the training data folder and record the annotation data in a single file.
                    let mut mda_anno_map = get_train_path_and_anno_content(
                        anno_data,
                        config.start.unwrap_or(0),
                        config.end.unwrap_or(0),
                    );

                    for item in &mut mda_anno_map {
                        let extract_name = extract_file_name(&item.file_name) + ".mda";
                        let mda_name = mda.to_owned() + &extract_name;
                        item.file_name = mda_name;
                    }
                    let pb = ProgressBar::new(mda_anno_map.len() as u64);

                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(config.threads.unwrap_or(10))
                        .build()
                        .unwrap();

                    pool.install(|| {
                        mda_anno_map.par_iter().for_each(
                            |item| match update_anno_in_mda_by_content(&item.file_name, anno_data) {
                                Ok(_) => {
                                    pb.inc(1);
                                }
                                Err(err) => {
                                    eprintln!("Failed to read data from MDA file: {}", err);
                                }
                            },
                        );
                    });
                    pb.finish_with_message("done");
                    mda_anno_map.len()
                } else {
                    eprintln!("{}", message::INVALID_PATH_MSG);
                    0
                }
            }

            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        };
        // Record end time
        record_end_time(start_time, number_of_mda_files, "updated");
    } else if config.action == "version" {
        match &config.mda {
            Some(mda) => {
                if is_file(mda) {
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
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        }
    } else if config.action == "extract" {
        // Record start time
        let start_time = record_start_time(&config.action);
        let format: String = config.format.unwrap_or("txt".to_string());

        // Extract Data
        let number_of_mda_files = match (&config.train, &config.anno, &config.mda) {
            // Extract anno data into different files
            (Some(train_data), Some(anno_data), Some(mda)) => {
                let anno_version: i32 = config.rev.unwrap_or_default();
                if is_directory(mda) && is_directory(train_data) && is_directory(anno_data) {
                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    let pb = ProgressBar::new(mda_files.len() as u64);

                    for file in &mda_files {
                        let train_data = train_data.to_string() + &extract_file_name(file);
                        let anno_data: String = anno_data.to_string() + &extract_file_name(file);
                        let _ = extract_data_from_mda(
                            file,
                            &train_data,
                            &anno_data,
                            anno_version,
                            &format,
                        );
                        pb.inc(1);
                    }
                    pb.finish_with_message("done");

                    mda_files.len()
                } else if is_file(mda) && is_directory(train_data) && is_directory(anno_data) {
                    let pb = ProgressBar::new(1);

                    let train_data = train_data.to_string() + &extract_file_name(mda);
                    let anno_data: String = anno_data.to_string() + &extract_file_name(mda);
                    let _ =
                        extract_data_from_mda(mda, &train_data, &anno_data, anno_version, &format);
                    pb.inc(1);

                    pb.finish_with_message("done");

                    1
                } else if is_file(anno_data) && is_directory(mda) && is_directory(train_data) {
                    let anno_version: i32 = config.rev.unwrap_or_default();

                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    let pb = ProgressBar::new(mda_files.len() as u64);

                    let mut content = String::new();
                    for file in &mda_files {
                        let train_data = train_data.to_string() + &extract_file_name(file);
                        let anno = extract_data_from_mda_and_anno_in_one_file(
                            file,
                            &train_data,
                            anno_version,
                        );
                        let (data_type, anno) = match anno {
                            Ok(anno) => anno,
                            Err(_) => {
                                std::process::exit(0);
                            }
                        };
                        let anno_content = anno.join(" ");
                        let mut train_name = String::new();
                        match data_type {
                            DataType::Text => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".txt";
                            }
                            DataType::Image => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".jpg";
                            }
                            DataType::Video => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".mp4";
                            }
                            DataType::Audio => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".wav";
                            }
                        }
                        let one_line = train_name + " " + &anno_content;

                        content = content + &one_line + "\n";
                        pb.inc(1);
                    }
                    pb.finish_with_message("done");
                    let mut file = match File::create(anno_data) {
                        Ok(file) => file,
                        Err(err) => {
                            eprintln!("Error creating file: {}", err);
                            std::process::exit(0);
                        }
                    };

                    match file.write_all(content.as_bytes()) {
                        Ok(_) => println!("\nFile write successful!"),
                        Err(err) => eprintln!("Error writing to file: {}", err),
                    }

                    mda_files.len()
                } else {
                    eprintln!("{}", message::INVALID_PATH_MSG);
                    0
                }
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        };

        // Record end time
        record_end_time(start_time, number_of_mda_files, "extracted");
    } else if config.action == "generate_many" {
        // Record start time
        let start_time = record_start_time(&config.action);
        // Generate mda files
        let number_of_mda_files = match (&config.train, &config.anno_config, &config.output) {
            (Some(train_data), Some(anno_config), Some(output)) => {
                //读取anno配置
                generate_mda_v1(&train_data, &anno_config, &output, &config)?;
                0
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        };
        // Record end time
        record_end_time(start_time, number_of_mda_files, "generated");
    } else if config.action == "group" {
        match &config.mda {
            Some(mda) => {
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();
                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);

                    let mut table = print_table_header();

                    for file in mda_files {
                        match read_info_from_mda(&file) {
                            Ok((index, header)) => {
                                table = print_table_cell(table.clone(), index, header);
                            }
                            Err(err) => {
                                eprintln!(
                                    "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                    mda,
                                    message::FAIL_TO_READ,
                                    err
                                );
                            }
                        }
                    }
                    table.printstd();
                } else if is_file(mda) {
                    match read_anno_groups_from_mda(mda) {
                        Ok((groups)) => {
                            println!("{:?}", groups);
                        }
                        Err(err) => {
                            eprintln!(
                                "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                                mda,
                                message::FAIL_TO_READ,
                                err
                            );
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
    } else if config.action == "version_1" {
        let group = match config.group {
            Some(group) => {
                if group == "NONE" {
                    eprintln!("Please input group");
                    std::process::exit(0);
                }
                group
            }
            None => {
                eprintln!("Please input group");
                std::process::exit(0);
            }
        };

        match &config.mda {
            Some(mda) => {
                if is_file(mda) {
                    match read_anno_from_mda_v1(mda, &group, -1) {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("Failed to read data from MDA file: {}", err);
                        }
                    }
                } else {
                }
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        }
    } else if config.action=="extract_1"{
        let group = match config.group {
            Some(group) => {
                if group == "NONE" {
                    eprintln!("Please input group");
                    std::process::exit(0);
                }
                group
            }
            None => {
                eprintln!("Please input group");
                std::process::exit(0);
            }
        };
        let number_of_mda_files = match (&config.train, &config.anno, &config.mda) {
            // Extract anno data into different files
            (Some(train_data), Some(anno_data), Some(mda)) => {
                let anno_version: i32 = config.rev.unwrap_or_default();
                if is_file(mda) && is_directory(train_data) && is_directory(anno_data) {
                    let pb = ProgressBar::new(1);

                    let train_data = train_data.to_string() + &extract_file_name(mda);
                    let anno_data: String = anno_data.to_string() + &extract_file_name(mda);
                    let _ =
                        extract_data_from_mda_v1(mda, &train_data, &anno_data, anno_version, "txt",&group);
                    pb.inc(1);

                    pb.finish_with_message("done");

                    1
                } else if is_file(anno_data) && is_directory(mda) && is_directory(train_data) {
                    let anno_version: i32 = config.rev.unwrap_or_default();

                    let mut mda_files: Vec<String> = Vec::new();

                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);
                    let pb = ProgressBar::new(mda_files.len() as u64);

                    let mut content = String::new();
                    for file in &mda_files {
                        let train_data = train_data.to_string() + &extract_file_name(file);
                        let anno = extract_data_from_mda_and_anno_in_one_file(
                            file,
                            &train_data,
                            anno_version,
                        );
                        let (data_type, anno) = match anno {
                            Ok(anno) => anno,
                            Err(_) => {
                                std::process::exit(0);
                            }
                        };
                        let anno_content = anno.join(" ");
                        let mut train_name = String::new();
                        match data_type {
                            DataType::Text => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".txt";
                            }
                            DataType::Image => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".jpg";
                            }
                            DataType::Video => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".mp4";
                            }
                            DataType::Audio => {
                                let name = extract_file_name(&train_data);
                                train_name = name + ".wav";
                            }
                        }
                        let one_line = train_name + " " + &anno_content;

                        content = content + &one_line + "\n";
                        pb.inc(1);
                    }
                    pb.finish_with_message("done");
                    let mut file = match File::create(anno_data) {
                        Ok(file) => file,
                        Err(err) => {
                            eprintln!("Error creating file: {}", err);
                            std::process::exit(0);
                        }
                    };

                    match file.write_all(content.as_bytes()) {
                        Ok(_) => println!("\nFile write successful!"),
                        Err(err) => eprintln!("Error writing to file: {}", err),
                    }

                    mda_files.len()
                } else {
                    eprintln!("{}", message::INVALID_PATH_MSG);
                    0
                }
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        };

    } else {
        println!(
            "\x1b[38;5;208m[WARN]\x1b[0m Wrong action! Support 5 actions for MDA: generate, list, update, version, extract!\n- generate: generate mda files for data.\n- list: list basic info of mda files\n- update: update the annotation data in mda files\n- version: list all versions of mda files\n- extract: extract training data and annotation data from mda files"
        );
    }
    Ok(())
}
