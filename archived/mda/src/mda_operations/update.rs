use crate::extract:: get_all_rev_anno_with_id  ;
use crate::{extract_file_name, MDAIndex};
use crate::generate::{
    write_mda_data, AnnoItem, Annotation, RevAnnoWithID, TrainMapAnno,
};
use crate::read_from_file::get_train_path_and_anno_content;
 
use crate::{  DataType, MDAHeader,   RevAnno, TrainingData};
use anyhow::Result;
use indicatif::ProgressBar;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::process;
/// update anno in combined anno file
pub fn update_anno_in_combined_file(
    mda: &str,
    anno_data: &str,
    // config: MDAOptions
    start: usize,
    end: usize,
    group: &str,
) -> Result<usize, Box<dyn Error>> {
    let mut mda_anno_map = get_train_path_and_anno_content(anno_data, start, end);
    
    for item in &mut mda_anno_map {
        let extract_name = extract_file_name(&item.file_name) + ".mda";
        let mda_name = mda.to_owned() + &extract_name;
        item.file_name = mda_name;
    }
    let train_map_anno = TrainMapAnno {
        id: group.to_string(),
        data: mda_anno_map,
    };
    // group the train and anno
    let mut anno_groups: Vec<Annotation> = Vec::new();

    let id = train_map_anno.id.clone();
    let data = &train_map_anno.data;

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
    let pb: ProgressBar = ProgressBar::new(anno_groups.len() as u64);
    let mut count=0;
    for item in anno_groups {
        // get previous data
        // Get rev_anno
        let mut rev_anno_ids: Vec<RevAnnoWithID> =
            match get_all_rev_anno_with_id(&item.file_name, -1) {
                Ok(rev_anno) => rev_anno,
                Err(err) => {
                    println!("Update annotation data fail! ={:?}", err);
                    process::exit(1);
                }
            };
         //update data
        for data in &mut rev_anno_ids {
            if group == data.id {
                let rev_anno = RevAnno::add_element(
                    &item.groups[0].content,
                    data.clone().rev_anno.entries,
                    data.clone().rev_anno.headers,
                );
                data.rev_anno = rev_anno;
            }
        }
        update_rev_anno(&item.file_name, &mut rev_anno_ids)?;
        pb.inc(1);
        count+=1;

    }
    pb.finish_with_message("done");

    Ok(count )
 }
/// Update anno rev_anno and offset in mda
pub fn update_anno_in_mda(
    file_path: &str,
    anno_data_path: &str,
    group: &str,
) -> Result<(), Box<dyn Error>> {
    // Get rev_anno
    let mut rev_anno_ids: Vec<RevAnnoWithID> = match get_all_rev_anno_with_id(file_path, -1) {
        Ok(rev_anno) => rev_anno,
        Err(err) => {
            println!("Update annotation data fail! ={:?}", err);
            process::exit(1);
        }
    };

    // // Rewrite new data
    let mut anno = File::open(anno_data_path)?;
    let mut content = String::new();
    anno.read_to_string(&mut content)?;

    // find the changed data
    for item in &mut rev_anno_ids {
        if item.id == group {
            let rev_anno = RevAnno::add_element(
                &content,
                item.clone().rev_anno.entries,
                item.clone().rev_anno.headers,
            );
            item.rev_anno = rev_anno;
            break;
        }
    }

    update_rev_anno(file_path, &mut rev_anno_ids)?;

    Ok(())
}

/// Update rev_anno
fn update_rev_anno(
    mda_path: &str,
    rev_anno: &mut [RevAnnoWithID],
) -> Result<(), Box<dyn Error>> {
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

    let _ = write_mda_data(mda_path, header, train_data, rev_anno);

    Ok(())
}
