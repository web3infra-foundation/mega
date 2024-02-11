use anyhow::Result;
use prettytable::{Cell, Row, Table};

use crate::extract::{
    extract_mda, extract_mda_more, read_anno_from_mda, read_anno_groups_from_mda,
};
use crate::generate::{
    generate_mda_combined_annotations, generate_mda_separate_annotation_one_to_one,
    generate_mda_separate_annotation_one_to_one_in_folder,
    generate_mda_separate_annotations_one_to_many,
    generate_mda_separate_annotations_one_to_many_in_folder,
};
use crate::read_from_folders::*;
use crate::update::update_anno_in_combined_file;
use crate::*;
use crate::{extract::read_info_from_mda, update::update_anno_in_mda};
use clap::Parser;
use indicatif::ProgressBar;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

use std::path::Path;
/// Command Line Tool
#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "", long_about = "Design and Implementation of File for AI Training Data", after_help = "")]
#[derive(Deserialize, Serialize)]
pub struct MDAOptions {
    /// 5 actions: generate, extract, list, group, version, update
    #[arg(long)]
    pub action: String,

    /// Training data file/folder path.
    #[arg(long)]
    pub train: Option<String>,

    /// Annotation data file/folder path.
    #[arg(long)]
    pub anno: Option<String>,

    /// Annotation data file/folder path, separated by commas
    #[arg(long)]
    pub annos: Option<String>,

    /// Output data file/folder path.
    #[arg(long)]
    pub output: Option<String>,

    /// MDA data file/folder path.
    #[arg(long)]
    pub mda: Option<String>,

    /// Tags for MDA files
    #[arg(long)]
    pub tags: Option<String>,

    /// Maximum number of threads  
    #[arg(long, default_value = "10")]
    pub threads: Option<usize>,

    /// The version of MDA file
    #[arg(long, default_value = "-1")]
    pub rev: Option<i32>,

    /// Read from which line of the annotation file.
    #[arg(long, default_value = "1")]
    pub start: Option<usize>,

    /// Read from which line of the annotation file.
    #[arg(long, default_value = "0")]
    pub end: Option<usize>,

    /// The type of the annotation data: txt,json
    #[arg(long, default_value = "txt")]
    pub format: Option<String>,

    /// Combined Annotation data config.
    #[arg(long)]
    pub anno_config: Option<String>,

    /// The group of the annotation data
    #[arg(long, default_value = "NONE")]
    pub group: Option<String>,

    /// The generation mode: one, multiple, combine
    #[arg(long)]
    pub mode: Option<String>,
 
}
#[allow(unused_assignments)]
pub fn run(config: MDAOptions) -> Result<(), Box<dyn Error>> {
    // Generate .mda file
    if config.action == "list" {
        match &config.mda {
            Some(mda) => {
                if is_directory(mda) {
                    let mut mda_files: Vec<String> = Vec::new();
                    find_mda_files_in_dir(Path::new(mda), &mut mda_files);

                    let mut table = print_table_header();

                    for file in mda_files {
                        match read_info_from_mda(&file) {
                            Ok((index, header)) => {
                                table = print_table_cell(&file,table.clone(), index, header);
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
                            let table = print_table_cell(mda,table, index, header);
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
                                    &group,
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
                    match update_anno_in_mda(mda, anno_data, &group) {
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

                    
                    match update_anno_in_combined_file(
                        mda,
                        anno_data,
                        config.start.unwrap_or(1),
                        config.end.unwrap_or(0),
                        &group,
                    ) {
                        Ok(data) => data,
                        Err(err) => {
                            eprintln!("{} {}", message::INVALID_PATH_MSG, err);
                            std::process::exit(0);
                        }
                    }
                    
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
        let group = match config.group {
            Some(group) => {
                if group == "NONE" {
                    eprintln!("Please input annotation group!");
                    std::process::exit(0);
                }
                group
            }
            None => {
                eprintln!("Please input annotation group!");
                std::process::exit(0);
            }
        };

        match &config.mda {
            Some(mda) => {
                if is_file(mda) {
                    match read_anno_from_mda(mda, &group, -1) {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("Failed to read data from MDA file: {}", err);
                        }
                    }
                } 
            }
            _ => {
                eprintln!("{}", message::INVALID_PATH_MSG);
                std::process::exit(0);
            }
        }
 
    } else if config.action == "group" {
        match &config.mda {
            Some(mda) => match read_anno_groups_from_mda(mda) {
                Ok(groups) => {
                    let mut table = Table::new();
                    table.add_row(Row::new(vec![
                        Cell::new("ID"),
                        Cell::new("Annotation Group"),
                    ]));
                    let mut count = 1;
                    for item in groups {
                        table.add_row(Row::new(vec![
                            Cell::new(&count.to_string()),
                            Cell::new(&item.to_string()),
                        ]));
                        count += 1;
                    }
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
            },
            _ => {
                println!("Please input mda file");
                std::process::exit(0);
            }
        }
    } else if config.action == "extract" {
        // Record start time
        let start_time = record_start_time(&config.action);
        let format: String = config.format.unwrap_or("txt".to_string());
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
        let threads = config.threads.unwrap_or(10);
        let number_of_mda_files = match (&config.train, &config.anno, &config.mda) {
            // Extract anno data into different files
            (Some(train_data), Some(anno_data), Some(mda)) => {
            

                let anno_version: i32 = config.rev.unwrap_or_default();
                if is_file(mda) && is_directory(train_data) && is_directory(anno_data) {
                   

                     extract_mda(mda, train_data, anno_data, anno_version, &format, &group)?;

                    1
                } else if is_directory(mda) && is_directory(anno_data) {
                  

                    extract_mda_more(
                        mda,
                        train_data,
                        anno_data,
                        anno_version,
                        &format,
                        &group,
                        threads,
                    )?;
                    1
                }else {
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
    } else if config.action == "generate" {
        // Record start time
        let start_time = record_start_time(&config.action);

        // Generate mda files
        let mut number_of_mda_files = 0;
        let mode = config.mode.as_ref().unwrap_or(&"NONE".to_string()).clone();
        if mode == "combine" {
            number_of_mda_files = match (&config.train, &config.anno_config, &config.output) {
                (Some(train_data), Some(anno_config), Some(output)) => {
                    match generate_mda_combined_annotations(
                        train_data,
                        anno_config,
                        output,
                        &config,
                    ) {
                        Ok(data) => data,
                        Err(err) => {
                            eprintln!("{} ERROR= {}", message::GENERATE_MSG, err);
                            std::process::exit(0);
                        }
                    }
                }
                _ => {
                    eprintln!("{}", message::INVALID_PATH_MSG);
                    std::process::exit(0);
                }
            };
        } else if mode == "one" {
            number_of_mda_files = match (&config.train, &config.anno, &config.output) {
                (Some(train_path), Some(anno_path), Some(output)) => {
                    if is_file(train_path) && is_file(anno_path) {
                        generate_mda_separate_annotation_one_to_one(
                            train_path, anno_path, output, &config,
                        )?;
                        1
                    } else if is_directory(train_path) && is_directory(anno_path) {
                        match generate_mda_separate_annotation_one_to_one_in_folder(
                            train_path, anno_path, output, &config,
                        ) {
                            Ok(data) => data,
                            Err(err) => {
                                eprintln!("{} ERROR= {}", message::GENERATE_MSG, err);
                                std::process::exit(0);
                            }
                        }
                    } else {
                        0
                    }
                }
                _ => {
                    eprintln!("{} {}", message::GENERATE_MSG, message::INVALID_PATH_MSG);
                    std::process::exit(0);
                }
            };
        } else if mode == "multiple" {
            number_of_mda_files = match (&config.train, &config.annos, &config.output) {
                (Some(train_path), Some(anno_group), Some(output)) => {
                    if is_file(train_path) {
                        generate_mda_separate_annotations_one_to_many(
                            train_path, anno_group, output, &config,
                        )?;
                        1
                    } else if is_directory(train_path) {
                        match generate_mda_separate_annotations_one_to_many_in_folder(
                            train_path, anno_group, output, &config,
                        ) {
                            Ok(data) => data,
                            Err(err) => {
                                eprintln!("{} ERROR= {}", message::GENERATE_MSG, err);
                                std::process::exit(0);
                            }
                        }
                    } else {
                        0
                    }
                }
                _ => {
                    eprintln!("{} {}", message::GENERATE_MSG, message::INVALID_PATH_MSG);
                    std::process::exit(0);
                }
            };
        } else {
            eprintln!(
                "{} Please input the correct generate mode!",
                message::GENERATE_MSG,
                
            );
            std::process::exit(0);
        }

        // Record end time
        record_end_time(start_time, number_of_mda_files, "generated");
    } else {
        println!(
            "\x1b[38;5;208m[WARN]\x1b[0m Wrong action! Support 5 actions for MDA: generate, list, update, version, extract!\n- generate: generate mda files for data.\n- list: list basic info of mda files\n- update: update the annotation data in mda files\n- version: list all versions of mda files\n- extract: extract training data and annotation data from mda files"
        );
    }
    Ok(())
}
