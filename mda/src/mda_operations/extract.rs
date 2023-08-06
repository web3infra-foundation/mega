use crate::{
    find_nearest_multiple_of_snapshot_base, get_full_data, print_rev_anno_headers,
    save_audio_to_file, save_image_to_file, save_text_to_file, save_video_to_file,
    write_strings_to_file, DataType, MDAHeader, MDAIndex, RevAnno, RevAnnoEntry, RevAnnoHeader,
};
use anyhow::Result;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::process;

/// Read data from an MDA file.
pub fn read_info_from_mda(file_path: &str) -> Result<(MDAIndex, MDAHeader), Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let index: MDAIndex = bincode::deserialize_from(&mut reader)?;
    reader.seek(SeekFrom::Start(index.header_offset))?;
    let header: MDAHeader = bincode::deserialize_from(&mut reader)?;
    Ok((index, header))
}

// Read annotations from an MDA file
pub fn read_anno_from_mda(file_path: &str, rev: i32) -> Result<(), Box<dyn Error>> {
    let rev_anno = match get_rev_anno_from_mda(file_path, rev) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("error={:?}", err);
            process::exit(1);
        }
    };
    println!("Data Version for {:?}", file_path);
    print_rev_anno_headers(&rev_anno.headers);
 
    Ok(())
}

/// Extract data from an MDA file.
pub fn extract_data_from_mda(
    mda_path: &str,
    training_data_path: &str,
    anno_data_path: &str,
    rev: i32,
    format:&str
) -> Result<DataType, Box<dyn Error>> {
    let _ = extract_anno_from_mda(mda_path, anno_data_path, rev,format);
    extract_train_from_mda(mda_path, training_data_path)
}

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
fn extract_anno_from_mda(
    file_path: &str,
    anno_data_path: &str,
    rev: i32,
    format:&str
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

    write_strings_to_file(&strings, anno_data_path,format)?;
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
