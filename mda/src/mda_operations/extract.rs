 use crate::{
    extract_file_name, find_nearest_multiple_of_snapshot_base, get_full_data, message,
    print_rev_anno_headers, save_audio_to_file, save_image_to_file, save_text_to_file,
    save_video_to_file, write_strings_to_file, DataType, MDAHeader, MDAIndex, MDAIndexTest,
    RevAnno, RevAnnoEntry, RevAnnoHeader,
};
use anyhow::Result;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::process;
/// Read data from an MDA file.
pub fn read_info_from_mda(file_path: &str) -> Result<(MDAIndexTest, MDAHeader), Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let index: MDAIndexTest = bincode::deserialize_from(&mut reader)?;
    reader.seek(SeekFrom::Start(index.header_offset))?;
    let header: MDAHeader = bincode::deserialize_from(&mut reader)?;
    Ok((index, header))
}
/// Get anno groups
pub fn read_anno_groups_from_mda(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let index: MDAIndexTest = bincode::deserialize_from(&mut reader)?;
    let mut anno_groups = Vec::new();
    for item in index.annotations_offset {
        anno_groups.push(item.clone().id);
    }
    Ok(anno_groups)
}

// Read annotations from an MDA file
pub fn read_anno_from_mda(file_path: &str, group: &str, rev: i32) -> Result<(), Box<dyn Error>> {
    let rev_anno = match get_rev_anno_from_mda_v1(file_path, group, rev) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("Read Version Fail = {:?}", err);
            process::exit(1);
        }
    };
    println!("Data Version for {:?}, anno group: {:?}", file_path, group);
    print_rev_anno_headers(&rev_anno.headers);

    Ok(())
}
// // Read annotations from an MDA file
// pub fn read_anno_from_mda(file_path: &str, rev: i32) -> Result<(), Box<dyn Error>> {
//     let rev_anno = match get_rev_anno_from_mda(file_path, rev) {
//         Ok(rev_anno) => rev_anno,
//         Err(err) => {
//             println!("error={:?}", err);
//             process::exit(1);
//         }
//     };
//     println!("Data Version for {:?}", file_path);
//     print_rev_anno_headers(&rev_anno.headers);

//     Ok(())
// }

/// Extract data from an MDA file.
// pub fn extract_data_from_mda(
//     mda_path: &str,
//     training_data_path: &str,
//     anno_data_path: &str,
//     rev: i32,
//     format: &str,
// ) -> Result<DataType, Box<dyn Error>> {
//     let _ = extract_anno_from_mda(mda_path, anno_data_path, rev, format);
//     extract_train_from_mda(mda_path, training_data_path)
// }

/// Extract data from an MDA file.
pub fn extract_data_from_mda_and_anno_in_one_file(
    mda_path: &str,
    training_data_path: &str,

    rev: i32,
) -> Result<(DataType, Vec<String>), Box<dyn Error>> {
    let data_type = match extract_train_from_mda(mda_path, training_data_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Fail to extract training data {:?}", e);
            process::exit(1);
        }
    };

    match extract_anno_from_mda_and_anno_in_one_file(mda_path, rev) {
        Ok(data) => Ok((data_type, data)),
        Err(e) => {
            eprintln!("Fail to extract anno data {:?}", e);

            process::exit(1);
        }
    }
}

/// Extract anno data from mda
#[allow(unused_assignments)]
fn extract_anno_from_mda_and_anno_in_one_file(
    file_path: &str,

    rev: i32,
) -> Result<Vec<String>, Box<dyn Error>> {
    let rev_anno = match get_rev_anno_from_mda(file_path, rev) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("error={:?}", err);
            process::exit(1);
        }
    };

    let mut full_data: String = String::new();
    if rev == -1 {
        let rev1 = rev_anno.entries.len() - 1;
        full_data = get_full_data(rev1 as i32, rev_anno.entries);
    } else {
        full_data = get_full_data(rev, rev_anno.entries);
    }

    let strings: Vec<String> = vec![full_data.to_string()];

    Ok(strings)
}

/// Extract anno data from mda
#[allow(unused_assignments)]
fn _extract_anno_from_mda(
    file_path: &str,
    anno_data_path: &str,
    rev: i32,
    format: &str,
) -> Result<(), Box<dyn Error>> {
    let rev_anno = match get_rev_anno_from_mda(file_path, rev) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("error={:?}", err);
            process::exit(1);
        }
    };

    let mut full_data: String = String::new();
    if rev == -1 {
        let rev1 = rev_anno.entries.len() - 1;
        full_data = get_full_data(rev1 as i32, rev_anno.entries);
    } else {
        full_data = get_full_data(rev, rev_anno.entries);
    }

    let strings: Vec<String> = vec![full_data.to_string()];

    write_strings_to_file(&strings, anno_data_path, format)?;
    Ok(())
}

/// Extract training data from mda
fn extract_train_from_mda(
    mda_path: &str,
    training_data_path: &str,
) -> Result<DataType, Box<dyn Error>> {
    let file = File::open(mda_path)?;
    let mut reader = BufReader::new(file);
    let index: MDAIndex = bincode::deserialize_from(&mut reader)?;

    reader.seek(SeekFrom::Start(index.train_data_offset))?;
    let data_type: DataType = bincode::deserialize_from(&mut reader)?;
    match data_type {
        DataType::Text => {
            let text: String = bincode::deserialize_from(&mut reader)?;

            save_text_to_file(&text, training_data_path)?;
        }
        DataType::Image => {
            let image_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;

            save_image_to_file(&image_data, training_data_path)?;
        }
        DataType::Video => {
            let video_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;

            save_video_to_file(&video_data, training_data_path)?;
        }
        DataType::Audio => {
            let audio_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;

            save_audio_to_file(&audio_data, training_data_path)?;
        }
    };

    Ok(data_type)
}

// Function to retrieve the annotation revision log from an MDA file
#[allow(unused_assignments)]
pub fn get_rev_anno_from_mda(file_path: &str, rev: i32) -> Result<RevAnno, Box<dyn Error>> {
    let mut rev = rev;

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // Deserialize the MDAIndex structure from the file, which contains offsets for headers and entries
    let index: MDAIndex = bincode::deserialize_from(&mut reader)?;

    reader.seek(SeekFrom::Start(index.anno_headers_offset))?;

    // Read the bytes data of the header information
    let mut header_bytes = Vec::new();
    reader.read_to_end(&mut header_bytes)?;

    //

    let mut headers: Vec<RevAnnoHeader> = Vec::new();
    let entries_bytes = Vec::new();
    let mut rev_anno: RevAnno = RevAnno::new(headers.clone(), entries_bytes);

    if rev == -1 {
        let mut offset = 0;
        while offset < header_bytes.len() {
            let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
            headers.push(header.clone());
            offset += bincode::serialized_size(&header)? as usize;
        }

        // If the rev is -1, set it to the last header's index, otherwise, use the provided rev
        if rev == -1 {
            rev = (headers.len() - 1) as i32;
        }

        let header_number = rev + 1;

        headers = headers.into_iter().take(header_number as usize).collect();

        let headers: Vec<RevAnnoHeader> =
            headers.into_iter().take(header_number as usize).collect();
        reader.seek(SeekFrom::Start(index.anno_entries_offset))?;

        let mut entries_bytes = Vec::new();
        reader.read_to_end(&mut entries_bytes)?;
        let mut entries: Vec<RevAnnoEntry> = Vec::new();
        let mut offset = 0;
        for rev_anno_header in &headers {
            let entry_bytes = &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
            let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
            entries.push(entry);

            offset += rev_anno_header.length as usize;
        }

        rev_anno = RevAnno::new(headers, entries);
    } else {
        // is snapshot
        // is diff situation
        match find_nearest_multiple_of_snapshot_base(rev) {
            Some(nearest_rev) => {
                let mut offset = 0;
                while offset < header_bytes.len() {
                    let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;

                    if (header.rev >= nearest_rev) && (header.rev <= rev) {
                        headers.push(header.clone());

                        offset += bincode::serialized_size(&header)? as usize;
                    } else {
                        offset += bincode::serialized_size(&header)? as usize;
                    }
                }

                // seek from the snapshot
                reader.seek(SeekFrom::Start(headers[0].offset))?;

                let mut entries_bytes = Vec::new();
                reader.read_to_end(&mut entries_bytes)?;

                let mut entries: Vec<RevAnnoEntry> = Vec::new();
                let mut offset = 0;
                for rev_anno_header in &headers {
                    let entry_bytes =
                        &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
                    let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                    entries.push(entry);

                    offset += rev_anno_header.length as usize;
                }

                rev_anno = RevAnno::new(headers, entries);
            }
            None => {
                if rev == 0 {
                    let offset = 0;
                    // config header
                    let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;

                    headers.push(header.clone());

                    // config entry
                    reader.seek(SeekFrom::Start(header.clone().offset))?;
                    let mut entries_bytes = Vec::new();
                    reader.read_to_end(&mut entries_bytes)?;
                    let mut entries: Vec<RevAnnoEntry> = Vec::new();
                    let entry_bytes = &&entries_bytes[0..(header.clone().length as usize)];
                    let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                    entries.push(entry);

                    rev_anno = RevAnno::new(headers, entries);
                } else {
                    let mut offset = 0;
                    while offset < header_bytes.len() {
                        let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
                        if header.rev <= rev {
                            headers.push(header.clone());
                        }

                        offset += bincode::serialized_size(&header)? as usize;
                    }

                    // config entry
                    reader.seek(SeekFrom::Start(headers[0].clone().offset))?;

                    let mut entries_bytes = Vec::new();
                    reader.read_to_end(&mut entries_bytes)?;

                    let mut entries: Vec<RevAnnoEntry> = Vec::new();
                    let mut offset = 0;
                    for rev_anno_header in &headers {
                        let entry_bytes =
                            &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
                        let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                        entries.push(entry);

                        offset += rev_anno_header.length as usize;
                    }

                    rev_anno = RevAnno::new(headers, entries);
                }
            }
        }
    }
    Ok(rev_anno)
}

#[allow(unused_assignments)]
pub fn get_rev_anno_from_mda_v1(
    file_path: &str,
    group: &str,
    rev: i32,
) -> Result<RevAnno, Box<dyn Error>> {
    let mut rev = rev;

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // Deserialize the MDAIndex structure from the file, which contains offsets for headers and entries
    let index: MDAIndexTest = bincode::deserialize_from(&mut reader)?;
    let mut anno_headers_offset = 0;
    let mut anno_entries_offset = 0;
    let mut next_anno_entries_offset = 0;

    for (counter, item) in index.annotations_offset.iter().enumerate() {
        if item.id == group {
            anno_headers_offset = item.header_offset;
            anno_entries_offset = item.entries_offset;
            if counter == index.annotations_offset.len() - 1 {
                next_anno_entries_offset = 0;
            } else {
                //我想打印下一个item
                let next_item = &index.annotations_offset[counter + 1];
                next_anno_entries_offset = next_item.clone().entries_offset;
            }
            break;
        }
    }

    reader.seek(SeekFrom::Start(anno_headers_offset))?;

    // Read the bytes data of the header information
    let mut header_bytes = Vec::new();
    reader.read_to_end(&mut header_bytes)?;
    //
    //

    let mut headers: Vec<RevAnnoHeader> = Vec::new();
    let entries_bytes = Vec::new();
    let mut rev_anno: RevAnno = RevAnno::new(headers.clone(), entries_bytes);

    //

    //
    if rev == -1 {
        let mut offset = 0;
        if next_anno_entries_offset == 0 {
            while offset < header_bytes.len() {
                let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
                headers.push(header.clone());

                offset += bincode::serialized_size(&header)? as usize;
            }
        } else {
            let mut current_position = anno_headers_offset;

            while offset < header_bytes.len() && current_position < next_anno_entries_offset {
                let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
                headers.push(header.clone());

                offset += bincode::serialized_size(&header)? as usize;
                current_position += bincode::serialized_size(&header)?;
            }
        }

        // If the rev is -1, set it to the last header's index, otherwise, use the provided rev
        if rev == -1 {
            rev = (headers.len() - 1) as i32;
        }

        let header_number = rev + 1;

        headers = headers.into_iter().take(header_number as usize).collect();

        let headers: Vec<RevAnnoHeader> =
            headers.into_iter().take(header_number as usize).collect();
        reader.seek(SeekFrom::Start(anno_entries_offset))?;

        let mut entries_bytes = Vec::new();
        reader.read_to_end(&mut entries_bytes)?;
        let mut entries: Vec<RevAnnoEntry> = Vec::new();
        let mut offset = 0;

        for rev_anno_header in &headers {
            let entry_bytes = &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
            let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
            entries.push(entry);

            offset += rev_anno_header.length as usize;
        }

        rev_anno = RevAnno::new(headers, entries);
    } else {
        // is snapshot
        // is diff situation
        match find_nearest_multiple_of_snapshot_base(rev) {
            Some(nearest_rev) => {
                let mut offset = 0;
                while offset < header_bytes.len() {
                    let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;

                    if (header.rev >= nearest_rev) && (header.rev <= rev) {
                        headers.push(header.clone());

                        offset += bincode::serialized_size(&header)? as usize;
                    } else {
                        offset += bincode::serialized_size(&header)? as usize;
                    }
                }

                // seek from the snapshot
                reader.seek(SeekFrom::Start(headers[0].offset))?;

                let mut entries_bytes = Vec::new();
                reader.read_to_end(&mut entries_bytes)?;

                let mut entries: Vec<RevAnnoEntry> = Vec::new();
                let mut offset = 0;
                for rev_anno_header in &headers {
                    let entry_bytes =
                        &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
                    let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                    entries.push(entry);

                    offset += rev_anno_header.length as usize;
                }

                rev_anno = RevAnno::new(headers, entries);
            }
            None => {
                if rev == 0 {
                    let offset = 0;
                    // config header
                    let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;

                    headers.push(header.clone());

                    // config entry
                    reader.seek(SeekFrom::Start(header.clone().offset))?;
                    let mut entries_bytes = Vec::new();
                    reader.read_to_end(&mut entries_bytes)?;
                    let mut entries: Vec<RevAnnoEntry> = Vec::new();
                    let entry_bytes = &&entries_bytes[0..(header.clone().length as usize)];
                    let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                    entries.push(entry);

                    rev_anno = RevAnno::new(headers, entries);
                } else {
                    let mut offset = 0;
                    while offset < header_bytes.len() {
                        let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
                        if header.rev <= rev {
                            headers.push(header.clone());
                        }

                        offset += bincode::serialized_size(&header)? as usize;
                    }

                    // config entry
                    reader.seek(SeekFrom::Start(headers[0].clone().offset))?;

                    let mut entries_bytes = Vec::new();
                    reader.read_to_end(&mut entries_bytes)?;

                    let mut entries: Vec<RevAnnoEntry> = Vec::new();
                    let mut offset = 0;
                    for rev_anno_header in &headers {
                        let entry_bytes =
                            &&entries_bytes[offset..(offset + rev_anno_header.length as usize)];
                        let entry: RevAnnoEntry = bincode::deserialize(entry_bytes)?;
                        entries.push(entry);

                        offset += rev_anno_header.length as usize;
                    }

                    rev_anno = RevAnno::new(headers, entries);
                }
            }
        }
    }
    Ok(rev_anno)
}

use indicatif::ProgressBar;
pub fn extract_mda(
    mda_path: &str,
    training_data_path: &str,
    anno_data_path: &str,
    rev: i32,
    format: &str,
    group: &str,
) -> Result<(), Box<dyn Error>> {
    let pb = ProgressBar::new(1);

    let train_data = training_data_path.to_string() + &extract_file_name(mda_path);
    let anno_data: String = anno_data_path.to_string() + &extract_file_name(mda_path);
    match extract_data_from_mda(mda_path, &train_data, &anno_data, rev, format, group) {
        Ok(_) => {
            pb.inc(1);

            pb.finish_with_message("done");
        }
        Err(e) => {
            eprintln!("Extract Error:{:?}", e);
        }
    }

    Ok(())
}
pub fn extract_mda_more(
    mda_path: &str,
    training_data_path: &str,
    anno_data_path: &str,
    rev: i32,
    format: &str,
    group: &str,
    // config:&MDAOptions
    threads: usize,
) -> Result<(), Box<dyn Error>> {
    // get all paths
    let entries = fs::read_dir(mda_path)?;
    let entries_vec: Vec<_> = entries.collect();
    let length = entries_vec.len();

    let mut paths = Vec::new();
    for entry in entries_vec {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            paths.push(file_path.to_string_lossy().to_string());
        }
    }
    println!("{:?}", paths);
    let pb = ProgressBar::new(length.try_into().unwrap());
    // use thread pool to generate files
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    pool.install(|| {
        paths.par_iter().for_each(|path| {
            match extract_data_from_mda(
                path,
                training_data_path,
                anno_data_path,
                rev,
                format,
                group,
            ) {
                Ok(_) => {
                    pb.inc(1);
                }
                Err(err) => {
                    eprintln!(
                        "\x1b[31m[ERROR]{}: {} {}\x1b[0m",
                        path,
                        message::GENERATE_MSG,
                        err
                    );
                }
            }
        });
    });

    // for entry in entries_vec  {
    //     let entry = entry?;
    //     let file_path = entry.path();

    //     if file_path.is_file() {
    //         println!("{:?}",file_path);
    //         match extract_data_from_mda(
    //             &file_path.to_string_lossy(),
    //             training_data_path,
    //             anno_data_path,
    //             rev,
    //             format,
    //             group,
    //         ) {
    //             Ok(_) => {
    //                 pb.inc(1);
    //             }
    //             Err(e) => {
    //                 eprintln!("Extract Error:{:?}", e);
    //             }
    //         }
    //     }
    // }
    pb.finish_with_message("done");

    Ok(())
}
/// Extract data from an MDA file.
pub fn extract_data_from_mda(
    mda_path: &str,
    training_data_path: &str,
    anno_data_path: &str,
    rev: i32,
    format: &str,
    group: &str,
) -> Result<DataType, Box<dyn Error>> {
    let _ = extract_anno_from_mda_v1(mda_path, anno_data_path, rev, format, group);
    extract_train_from_mda(mda_path, training_data_path)
}

/// Extract anno data from mda
#[allow(unused_assignments)]
fn extract_anno_from_mda_v1(
    file_path: &str,
    anno_data_path: &str,
    rev: i32,
    format: &str,
    group: &str,
) -> Result<(), Box<dyn Error>> {
    let rev_anno = match get_rev_anno_from_mda_v1(file_path, group, rev) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("error={:?}", err);
            process::exit(1);
        }
    };

    let mut full_data: String = String::new();
    if rev == -1 {
        let rev1 = rev_anno.entries.len() - 1;
        full_data = get_full_data(rev1 as i32, rev_anno.entries);
    } else {
        full_data = get_full_data(rev, rev_anno.entries);
    }

    let strings: Vec<String> = vec![full_data.to_string()];

    write_strings_to_file(&strings, anno_data_path, format)?;
    Ok(())
}
