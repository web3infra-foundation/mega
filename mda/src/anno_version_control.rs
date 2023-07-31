//! Build a data structure similar to the revlog format to implement version control and incremental storage.

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::cmp::{max, min};
use std::process;

mod constants {
    pub const BLOCK_SIZE: usize = 10; // TODO
    pub const NULLID: [u8; 20] = [0; 20];
    pub const SNAPSHOT_BASE: u32 = 3;
}

///  Splitting large-scale data into fixed-size data blocks and recording the block numbers.
fn split_data_into_blocks(data: Vec<u8>, block_size: usize) -> (Vec<DataBlock>, Vec<usize>) {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut block_number = 0;
    let mut numbers: Vec<usize> = Vec::new();
    while index < data.len() {
        numbers.push(block_number);

        let end = std::cmp::min(index + block_size, data.len());
        blocks.push(DataBlock::new(block_number, data[index..end].to_vec()));
        index = end;
        block_number += 1;
    }

    (blocks, numbers)
}

/// Comparing data block lists to find newly added data blocks.
fn find_different_blocks(
    last_id: u8,
    entries: &Vec<RevAnnoEntry>,
    current_data: &[u8],
    _block_size: usize,
) -> Vec<DataBlock> {
    let blocks_list = get_data_blocks_up_to_id(last_id, entries);
    let (current_data_blocks, _data_indices) =
        split_data_into_blocks(current_data.clone().to_vec(), constants::BLOCK_SIZE);

    // Find elements in block1 that are not in block2
    let elements_not_in_block1: Vec<DataBlock> = current_data_blocks
        .iter()
        .filter(|current_data_blocks_item| {
            !blocks_list
                .iter()
                .any(|blocks_list_item| blocks_list_item.data == current_data_blocks_item.data)
        })
        .cloned()
        .collect();

    elements_not_in_block1
}

/// add new blocks to blocklist
fn add_to_block_list(
    mut block_list: Vec<DataBlock>,
    different_blocks: Vec<DataBlock>,
) -> (Vec<DataBlock>, Vec<usize>) {
    let mut diff_number = Vec::<usize>::new();
    for mut block in different_blocks {
        let last_block_number = block_list.last().map_or(0, |block| block.block_number);

        block.block_number = 1 + last_block_number;
        diff_number.push(block.block_number);
        block_list.push(block);
    }

    // block_list
    (block_list, diff_number)
}

/// extract index from data blocks
fn extract_index(vec_data1: &[DataBlock], vec_data2: &[DataBlock]) -> Vec<usize> {
    let mut index: Vec<usize> = Vec::new();
    for data_block1 in vec_data1.iter() {
        if let Some(index_in_vec_data2) = vec_data2
            .iter()
            .position(|data_block2| data_block1.data == data_block2.data)
        {
            index.push(vec_data2[index_in_vec_data2].block_number);
        }
    }

    index
}

impl RevAnnoEntry {
    /// new RevAnnoEntry
    fn new(id: u8, index: Vec<usize>, blocks: Vec<DataBlock>) -> Self {
        RevAnnoEntry { id, index, blocks }
    }

    /// add first RevAnnoEntry
    pub fn init(content: &str) -> (Vec<RevAnnoHeader>, Vec<RevAnnoEntry>) {
        // Config current content
        let data: Vec<u8> = content.as_bytes().to_vec();
        let (blocks, data_indices) = split_data_into_blocks(data.clone(), constants::BLOCK_SIZE);

        // Config enrty
        let entry = RevAnnoEntry::new(0, data_indices, blocks);

        let entries: Vec<RevAnnoEntry> = vec![entry];

        // Config Header
        let nodeid = compute_nodeid(&constants::NULLID, &constants::NULLID, &data);

        let rev_anno_header = RevAnnoHeader::new(
            0,
            0,
            data.len() as u32,
            0,
            0,
            constants::NULLID,
            constants::NULLID,
            nodeid,
            true
        );
        let headers: Vec<RevAnnoHeader> = vec![rev_anno_header];
        (headers, entries)
    }

    /// add entries to list
    pub fn add(
        content: &str,
        mut entries: Vec<RevAnnoEntry>,
        mut headers: Vec<RevAnnoHeader>,
    ) -> (Vec<RevAnnoHeader>, Vec<RevAnnoEntry>) {
        // 获取上一个entry
        //Config data from last entry
        let last_entry = entries.last().unwrap_or_else(|| {
            println!("The last data is empty!");
            process::exit(1);
        });
        let last_id = last_entry.id;
        let last_header = headers.last().unwrap_or_else(|| {
            println!("The last data is empty!");
            process::exit(1);
        });
        let last_node_id=last_header.nodeid;
        let mut last_p1 = last_header.p1rev;
        if last_id == 0 {
            last_p1 = last_header.nodeid;
        }

        // 配置现在的数据
        // Config current data info
        let current_id = last_id + 1;

        // change to Vec<u8>
        let current_data: Vec<u8> = content.as_bytes().to_vec();
        let (current_data_blocks, _data_indices) =
            split_data_into_blocks(current_data.clone(), constants::BLOCK_SIZE);

        // 找到不同的块
        // Build a block list and record the construction number of the original data
        let different_blocks =
            find_different_blocks(last_id, &entries, &current_data, constants::BLOCK_SIZE);

        let block_list = get_data_blocks_up_to_id(last_id, &entries);
        let (records, diff) = add_to_block_list(block_list, different_blocks);

        // assign id to diff blocks
        let diff_blocks: Vec<DataBlock> = records
            .iter()
            .filter_map(|record| {
                if diff.contains(&record.block_number) {
                    Some(DataBlock {
                        block_number: record.block_number,
                        data: record.data.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // get current index
        //配置现在entries的index
        let matching_block_numbers = extract_index(&current_data_blocks, &records);
        let store_matching_block_numbers = matching_block_numbers.clone();

        // 配置entry，1）如果已经存在，就不存
        //           2）如果是快照，全存
        //   3）否则就存diff
        // Config entry
        let nodeid = compute_nodeid(&constants::NULLID, &constants::NULLID, &current_data);

        let mut entry = RevAnnoEntry {
            id: current_id,
            index: matching_block_numbers,
            blocks: diff_blocks,
        };
        // 如果已经存在，就不存
        // Check if it existed
        for item in &entries {
            if item.index == entry.index {
                return (headers, entries.clone());
            }
        }
        //存diff
        entries.push(entry);

        // check if it is a snapshot

        // 如果是快照，全存
        if (current_id as u32) % constants::SNAPSHOT_BASE == 0 {
            let mut all_blocks: Vec<DataBlock> = Vec::new();
            for entry in &mut entries {
                for block in &entry.blocks {
                    all_blocks.push(block.clone());
                }
            }
            let new_entry = RevAnnoEntry {
                id: current_id,                      // Copy the id field
                index: store_matching_block_numbers, // Copy the index field
                blocks: all_blocks,                  // Use the cloned all_blocks here
            };
            entry = new_entry; // Assign the modified new_entry back to entry

            if let Some(last_element) = entries.last_mut() {
                *last_element = entry; // 修改倒数第一个元素的值
            }
            //Config header
            let rev_anno_header = RevAnnoHeader::new(
                current_id,
                0,
                0,
                current_id as i32,
                last_id as i32,
                last_node_id,
                constants::NULLID,
                nodeid,
                true
            );

            headers.push(rev_anno_header);
            (headers, entries)
        } else {
            // 如果是diff
             //Config header
             let mut rev_anno_header = RevAnnoHeader::new(
                current_id,
                0,
                0,
                0,
                last_id as i32,
                last_p1,
                constants::NULLID,
                nodeid,
                false
            );
            let nearest_id = find_nearest_multiple_of_snapshot_base(last_id as u32);
            match nearest_id {
                Some(nearest_id) => {
                    rev_anno_header.baserev=nearest_id as i32;
                  
                    if let Some(nearest_item)=headers.get(nearest_id as usize){
                        rev_anno_header.p2rev=nearest_item.nodeid;
                    }
                }
                None => {
                    rev_anno_header.baserev=current_id as i32;

                    
                }
            }

           

            headers.push(rev_anno_header);
             (headers, entries)
        }
    }
}

/// Compute nodeid hash using sha1
fn compute_nodeid(parent1: &[u8; 20], parent2: &[u8; 20], contents: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(min(parent1, parent2));
    hasher.update(max(parent1, parent2));
    hasher.update(contents);
    let result = hasher.finalize();
    let mut nodeid = [0u8; 20];
    nodeid.copy_from_slice(&result);
    nodeid
}

/// shorten nodeid
fn nodeid_to_short_hex(nodeid: &[u8; 20]) -> String {
    let nodeid_hex_string: String = nodeid
        .iter()
        .take(6)
        .map(|b| format!("{:02x}", b))
        .collect();
    nodeid_hex_string
}

/// Function to combine Vec<DataBlock> into text
fn combine_data_blocks_to_text(data_blocks: &Vec<DataBlock>) -> String {
    let mut combined_text = String::new();
    for data_block in data_blocks {
        combined_text.push_str(std::str::from_utf8(&data_block.data).unwrap());
    }
    combined_text
}

/// Find the corresponding indexes by ID.
fn find_index_by_id(id: u8, delta_list: &[RevAnnoEntry]) -> Option<Vec<usize>> {
    let delta_to_find = delta_list.iter().find(|entry| entry.id == id);

    delta_to_find.map(|entry| entry.index.clone())
}

/// Get all data blocks from ID 0 to the input ID.
fn get_data_blocks_up_to_id(last_id: u8, delta_list: &Vec<RevAnnoEntry>) -> Vec<DataBlock> {
    let mut data_blocks = Vec::new();
    let nearest_id = find_nearest_multiple_of_snapshot_base(last_id as u32);
    match nearest_id {
        Some(nearest_id) => {
            let mut delta_list_iter = delta_list.iter().skip_while(|entry| entry.id < nearest_id as u8);
            for entry in &mut delta_list_iter {
                data_blocks.extend(entry.blocks.iter().cloned());
            }
        }
        None => {
  
            for entry in delta_list {
                if entry.id <= last_id {
                    data_blocks.extend(entry.blocks.iter().cloned());
                }
            }
        }
    }
    data_blocks
}
fn find_nearest_multiple_of_snapshot_base(target: u32) -> Option<u32> {
    if target < constants::SNAPSHOT_BASE {
        return None; // 输入数太小，无法找到符合条件的结果
    }

    let nearest_multiple = target - (target % constants::SNAPSHOT_BASE); // 计算最小的 3 的倍数
    if nearest_multiple < constants::SNAPSHOT_BASE {
        return None; // 计算得到的结果小于 3，说明没有符合条件的结果
    }

    Some(nearest_multiple)
}

/// Get the Vec<DataBlock> corresponding to the indexes.
fn get_data_blocks_by_index(index: &Vec<usize>, data_blocks: &[DataBlock]) -> Vec<DataBlock> {
    let mut result_blocks = Vec::new();
    for &idx in index {
        if let Some(data_block) = data_blocks.iter().find(|block| block.block_number == idx) {
            result_blocks.push(data_block.clone());
        }
    }
    result_blocks
}
/// Get full data(string)
pub fn get_full_data(id: u8, entries: Vec<RevAnnoEntry>) -> String {
    if let Some(index) = find_index_by_id(id, &entries) {
        let data_blocks = get_data_blocks_up_to_id(id, &entries);
        let selected_blocks = get_data_blocks_by_index(&index, &data_blocks);
        combine_data_blocks_to_text(&selected_blocks)
    } else {
        println!("No data blocks found for ID {}", id);
        process::exit(1);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnnoEntry {
    pub id: u8,
    pub index: Vec<usize>,
    pub blocks: Vec<DataBlock>,
}
/// Structure for a data block
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DataBlock {
    /// Block number of the data block
    pub block_number: usize,
    /// Content of the data block
    pub data: Vec<u8>,
}

impl DataBlock {
    fn new(block_number: usize, data: Vec<u8>) -> Self {
        DataBlock { block_number, data }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnnoHeader {
    pub rev: u8,
    pub offset: u64,
    pub length: u32,
    pub baserev: i32,
    pub linkrev: i32,
    pub p1rev: [u8; 20],
    pub p2rev: [u8; 20],
    pub nodeid: [u8; 20],
    pub snapshot:bool
}
impl RevAnnoHeader {
    #![allow(clippy::too_many_arguments)]
    fn new(
        rev: u8,
        offset: u64,
        length: u32,
        baserev: i32,
        linkrev: i32,
        p1rev: [u8; 20],
        p2rev: [u8; 20],
        nodeid: [u8; 20],
        snapshot:bool
    ) -> RevAnnoHeader {
        RevAnnoHeader {
            rev: (rev),
            offset: (offset),
            length,
            baserev,
            linkrev,
            p1rev,
            p2rev,
            nodeid,
            snapshot
        }
    }
}

 
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnno {
 
    pub headers: Vec<RevAnnoHeader>,
    pub entries: Vec<RevAnnoEntry>,
}

impl RevAnno {
    pub fn new(rev_anno_header: Vec<RevAnnoHeader>, entries: Vec<RevAnnoEntry>) -> Self {
       
        RevAnno {
           
            headers: (rev_anno_header),
            entries: (entries),
        }
    }
}

pub fn print_rev_anno_headers(headers: &Vec<RevAnnoHeader>) {
    println!(
        "{:<6} {:<8} {:<7} {:<6} {:<7} {:<12} {:<12} {:<40} {:<6} ",
        "rev", "offset", "length", "delta", "linkrev", "nodeid", "p1", "p2","snap"
    );
    for (count, header) in headers.iter().enumerate() {
        let mut rev = header.rev.to_string();
        if count == headers.len() - 1 {
            rev = header.rev.to_string() + "*";
        }
        println!(
            "{:<6} {:<8} {:<7} {:<6} {:<7} {:<12} {:<12} {:<40} {:<6} ",
            rev,
            header.offset,
            header.length,
            header.baserev,
            header.linkrev,
            nodeid_to_short_hex(&header.nodeid),
            nodeid_to_short_hex(&header.p1rev),
            nodeid_to_short_hex(&header.p2rev),
            header.snapshot
        );
    }
}
