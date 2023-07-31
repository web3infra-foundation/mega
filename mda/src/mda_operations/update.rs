use crate::{
    generate::write_data_to_mda, DataType, MDAHeader, MDAIndex, RevAnno, RevAnnoEntry,
    RevAnnoHeader, TrainingData,
};
use anyhow::Result;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::{Seek, SeekFrom};

/// Update anno rev_anno and offset in mda
pub fn update_anno_in_mda(file_path: &str, anno_data_path: &str) -> Result<(), Box<dyn Error>> {
    // Get old rev_anno
    let   file = File::open(file_path)?;
    let mut reader = BufReader::new(&file);
    let index: MDAIndex = bincode::deserialize_from(&mut reader)?;
    reader.seek(SeekFrom::Start(index.anno_headers_offset))?;
   
    // Get MdaHeader info
    let mut header_bytes = Vec::new();
    reader.read_to_end(&mut header_bytes)?;

     let mut headers: Vec<RevAnnoHeader> = Vec::new();
    let mut offset = 0;
    while offset < header_bytes.len() {
        let header: RevAnnoHeader = bincode::deserialize(&header_bytes[offset..])?;
        headers.push(header.clone());

         offset += bincode::serialized_size(&header)? as usize;
    }

    // Move to entries offset
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
    
    // Rewrite new data
    let mut anno = File::open(anno_data_path)?;
    let mut content = String::new();
    anno.read_to_string(&mut content)?;
    let (headers, entries) = RevAnnoEntry::add(&content, entries, headers);
    let mut rev_anno = RevAnno::new(headers, entries);


    let _ = update_rev_anno(file_path, &mut rev_anno);
    

    Ok(())
}



/// Update rev_anno
fn update_rev_anno(mda_path: &str, rev_anno: &mut RevAnno) -> Result<(), Box<dyn Error>> {
    let file = File::open(mda_path)?;
    let mut reader = BufReader::new(file);
    let index: MDAIndex = bincode::deserialize_from(&mut reader)?;
    reader.seek(SeekFrom::Start(index.header_offset))?;

    let header: MDAHeader = bincode::deserialize_from(&mut reader)?;

    reader.seek(SeekFrom::Start(index.train_data_offset))?;

    let data_type: DataType = bincode::deserialize_from(&mut reader)?;

    let train_data: TrainingData = match data_type {
        DataType::Text => {
            let text: String = bincode::deserialize_from(&mut reader)?;
            TrainingData::Text(text.clone())
        }
        DataType::Image => {
            let image_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            TrainingData::Image(image_data.clone())
        }
        DataType::Video => {
            let video_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            TrainingData::Video(video_data.clone())
        }
        DataType::Audio => {
            let audio_data: Vec<u8> = bincode::deserialize_from(&mut reader)?;
            TrainingData::Audio(audio_data.clone())
        }
    };

    let _ = write_data_to_mda(mda_path, header, train_data, rev_anno);

    Ok(())
}
