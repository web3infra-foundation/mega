use crate::{
    extract::get_rev_anno_from_mda, generate::write_data_to_mda, DataType, MDAHeader, MDAIndex,
    RevAnno, TrainingData,
};
use anyhow::Result;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::process;
/// Update anno rev_anno and offset in mda
pub fn update_anno_in_mda_by_content(
    file_path: &str,
    anno_data_content: &str,
) -> Result<(), Box<dyn Error>> {
    //Get rev_anno

    let rev_anno = match get_rev_anno_from_mda(file_path, -1) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("Get annotation data fail! {:?}", err);
            process::exit(1);
        }
    };

    // Rewrite new data

    let mut rev_anno = RevAnno::add_element(anno_data_content, rev_anno.entries, rev_anno.headers);

    let _ = update_rev_anno(file_path, &mut rev_anno);

    Ok(())
}

/// Update anno rev_anno and offset in mda
pub fn update_anno_in_mda(file_path: &str, anno_data_path: &str) -> Result<(), Box<dyn Error>> {
    //Get rev_anno

    let rev_anno = match get_rev_anno_from_mda(file_path, -1) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("Update annotation data fail! ={:?}", err);
            process::exit(1);
        }
    };

    // Rewrite new data
    let mut anno = File::open(anno_data_path)?;
    let mut content = String::new();
    anno.read_to_string(&mut content)?;
    let mut rev_anno = RevAnno::add_element(&content, rev_anno.entries, rev_anno.headers);

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
