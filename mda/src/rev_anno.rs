//! Build a block_data structure similar to the revlog format to implement version control and incremental storage.
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::process;

mod constants {
    pub const BLOCK_SIZE: usize = 10;
    /// Snapshot Baseline Configuration
    pub const SNAPSHOT_BASE: i32 = 10;
}

/// Structure for a block
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Block {
    /// Block number of the block
    pub block_number: u64,
    /// Content of the block
    pub block_data: Vec<u8>,
}

impl Block {
    /// Create Block
    fn new(block_number: u64, block_data: Vec<u8>) -> Self {
        Block {
            block_number,
            block_data,
        }
    }
}

/// The Header of RevAnno
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnnoHeader {
    /// The number of the record
    pub rev: i32,
    /// The offset of the corresponding RevAnnoEntry
    pub offset: u64,
    /// The length of the corresponding RevAnnoEntry
    pub length: u64,
    /// Is a snapshot or not
    pub snapshot: bool,
}

impl RevAnnoHeader {
    #![allow(clippy::too_many_arguments)]
    fn new(rev: i32, offset: u64, length: u64, snapshot: bool) -> RevAnnoHeader {
        RevAnnoHeader {
            rev,
            offset: (offset),
            length,
            snapshot,
        }
    }
}

/// The entry of RevAnno
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnnoEntry {
    /// The number of the record
    pub rev: i32,
    /// The index of blocks
    pub index: Vec<u64>,
    /// The data block
    pub blocks: Vec<Block>,
}

impl RevAnnoEntry {
    /// new RevAnnoEntry
    fn new(rev: i32, index: Vec<u64>, blocks: Vec<Block>) -> Self {
        RevAnnoEntry { rev, index, blocks }
    }
}

/// The RevAnno object
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevAnno {
    /// The list of rev_anno_headers
    pub headers: Vec<RevAnnoHeader>,
    /// The list of rev_anno_entries
    pub entries: Vec<RevAnnoEntry>,
}

impl RevAnno {
    /// Create Object
    pub fn new(rev_anno_header: Vec<RevAnnoHeader>, entries: Vec<RevAnnoEntry>) -> Self {
        RevAnno {
            headers: (rev_anno_header),
            entries: (entries),
        }
    }
    /// add first RevAnnoEntry
    pub fn set_initial_element(content: &str) -> RevAnno {
        // Config current content
        let block_data: Vec<u8> = content.as_bytes().to_vec();
        let (blocks, data_indices) =
            split_data_into_blocks(block_data.clone(), constants::BLOCK_SIZE);

        // Config entries
        let entries: Vec<RevAnnoEntry> = vec![RevAnnoEntry::new(0, data_indices, blocks)];

        // Config headers
        let headers: Vec<RevAnnoHeader> = vec![RevAnnoHeader::new(0, 0, 0, true)];
        RevAnno { headers, entries }
    }

    /// add RevAnnoEntry
    pub fn add_element(
        content: &str,
        mut entries: Vec<RevAnnoEntry>,
        mut headers: Vec<RevAnnoHeader>,
    ) -> RevAnno {
        // Config block_data from last entry
        let last_entry = entries.last().unwrap_or_else(|| {
            eprintln!("AnnoDataError: Fail to add a RevAnnoEntry! The last block is empty!");
            process::exit(1);
        });
        let last_rev = last_entry.rev;

        // Config current block_data info
        let current_id = last_rev + 1;

        // change to Vec<u8>
        let current_data: Vec<u8> = content.as_bytes().to_vec();
        let (current_data_blocks, _data_indices) =
            split_data_into_blocks(current_data.clone(), constants::BLOCK_SIZE);

        // Build a block list and record the construction number of the original block_data
        let different_blocks =
            find_different_blocks(last_rev, &entries, &current_data, constants::BLOCK_SIZE);

        let block_list = get_data_blocks_up_to_id(last_rev, &entries);
        let (records, diff) = add_to_block_list(block_list, different_blocks);

        // assign rev to diff blocks
        let diff_blocks: Vec<Block> = records
            .iter()
            .filter_map(|record| {
                if diff.contains(&record.block_number) {
                    Some(Block {
                        block_number: record.block_number,
                        block_data: record.block_data.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // get current index

        let matching_block_numbers = extract_index(&current_data_blocks, &records);
        let store_matching_block_numbers = matching_block_numbers.clone();

        // Configure the entry:
        // 1) If it already exists, do not store it.
        // 2) If it is a snapshot, store the entire entry.
        // 3) Otherwise, store only the differential block_data.
        // Config entry

        let mut entry = RevAnnoEntry {
            rev: current_id,
            index: matching_block_numbers,
            blocks: diff_blocks,
        };

        // 1) Check if it existed
        for item in &entries {
            if item.index == entry.index {
                return RevAnno { headers, entries };
            }
        }

        entries.push(entry);

        // 2) Check if it is a snapshot
        if (current_id) % constants::SNAPSHOT_BASE == 0 {
            let mut all_blocks: Vec<Block> = Vec::new();
            for entry in &mut entries {
                for block in &entry.blocks {
                    all_blocks.push(block.clone());
                }
            }
            let new_entry = RevAnnoEntry {
                rev: current_id,                     // Copy the rev field
                index: store_matching_block_numbers, // Copy the index field
                blocks: all_blocks,                  // Use the cloned all_blocks here
            };
            entry = new_entry; // Assign the modified new_entry back to entry

            if let Some(last_element) = entries.last_mut() {
                *last_element = entry;
            }
            //Config header
            let rev_anno_header = RevAnnoHeader::new(current_id, 0, 0, true);
            headers.push(rev_anno_header);
        } else {
            // 3)
            //Config header
            let rev_anno_header = RevAnnoHeader::new(current_id, 0, 0, false);
            headers.push(rev_anno_header);
        }
        RevAnno { headers, entries }
    }
}

///  Splitting large-scale data into fixed-size data blocks and recording the block numbers.
fn split_data_into_blocks(block_data: Vec<u8>, block_size: usize) -> (Vec<Block>, Vec<u64>) {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut block_number = 0;
    let mut numbers: Vec<u64> = Vec::new();
    while index < block_data.len() {
        numbers.push(block_number);

        let end = std::cmp::min(index + block_size, block_data.len());
        blocks.push(Block::new(block_number, block_data[index..end].to_vec()));
        index = end;
        block_number += 1;
    }

    (blocks, numbers)
}

/// Comparing data block lists to find newly added data blocks.
fn find_different_blocks(
    last_rev: i32,
    entries: &Vec<RevAnnoEntry>,
    current_data: &[u8],
    _block_size: usize,
) -> Vec<Block> {
    let blocks_list = get_data_blocks_up_to_id(last_rev, entries);
    let (current_data_blocks, _data_indices) =
        split_data_into_blocks(current_data.clone().to_vec(), constants::BLOCK_SIZE);

    // Find elements in block1 that are not in block2
    let elements_not_in_block1: Vec<Block> = current_data_blocks
        .iter()
        .filter(|current_data_blocks_item| {
            !blocks_list.iter().any(|blocks_list_item| {
                blocks_list_item.block_data == current_data_blocks_item.block_data
            })
        })
        .cloned()
        .collect();

    elements_not_in_block1
}

/// add new blocks to blocklist
fn add_to_block_list(
    mut block_list: Vec<Block>,
    different_blocks: Vec<Block>,
) -> (Vec<Block>, Vec<u64>) {
    let mut diff_number = Vec::<u64>::new();
    for mut block in different_blocks {
        let last_block_number = block_list.last().map_or(0, |block| block.block_number);

        block.block_number = 1 + last_block_number;
        diff_number.push(block.block_number);
        block_list.push(block);
    }

    // block_list
    (block_list, diff_number)
}

/// extract index from block_data blocks
fn extract_index(vec_data1: &[Block], vec_data2: &[Block]) -> Vec<u64> {
    let mut index: Vec<u64> = Vec::new();
    for data_block1 in vec_data1.iter() {
        if let Some(index_in_vec_data2) = vec_data2
            .iter()
            .position(|data_block2| data_block1.block_data == data_block2.block_data)
        {
            index.push(vec_data2[index_in_vec_data2].block_number);
        }
    }

    index
}

/// Function to combine Vec<Block> into text
fn combine_data_blocks_to_text(data_blocks: &Vec<Block>) -> String {
    let mut combined_text = String::new();
    for data_block in data_blocks {
        combined_text.push_str(std::str::from_utf8(&data_block.block_data).unwrap());
    }
    combined_text
}

/// Find the corresponding indexes by ID.
fn find_index_by_id(rev: i32, delta_list: &[RevAnnoEntry]) -> Option<Vec<u64>> {
    let delta_to_find = delta_list.iter().find(|entry| entry.rev == rev);

    delta_to_find.map(|entry| entry.index.clone())
}

/// Get all block_data blocks from ID 0 to the input ID.
fn get_data_blocks_up_to_id(last_rev: i32, delta_list: &Vec<RevAnnoEntry>) -> Vec<Block> {
    let mut data_blocks = Vec::new();
    let nearest_id = find_nearest_multiple_of_snapshot_base(last_rev);
    match nearest_id {
        Some(nearest_id) => {
            let mut delta_list_iter = delta_list.iter().skip_while(|entry| entry.rev < nearest_id);
            for entry in &mut delta_list_iter {
                data_blocks.extend(entry.blocks.iter().cloned());
            }
        }
        None => {
            for entry in delta_list {
                if entry.rev <= last_rev {
                    data_blocks.extend(entry.blocks.iter().cloned());
                }
            }
        }
    }
    data_blocks
}
pub fn find_nearest_multiple_of_snapshot_base(target: i32) -> Option<i32> {
    if target < constants::SNAPSHOT_BASE {
        return None;
    }

    let nearest_multiple = target - (target % constants::SNAPSHOT_BASE);
    if nearest_multiple < constants::SNAPSHOT_BASE {
        return None;
    }

    Some(nearest_multiple)
}

/// Get the Vec<Block> corresponding to the indexes.
fn get_data_blocks_by_index(index: &Vec<u64>, data_blocks: &[Block]) -> Vec<Block> {
    let mut result_blocks = Vec::new();
    for &idx in index {
        if let Some(data_block) = data_blocks.iter().find(|block| block.block_number == idx) {
            result_blocks.push(data_block.clone());
        }
    }
    result_blocks
}
/// Get full block_data(string)
pub fn get_full_data(rev: i32, entries: Vec<RevAnnoEntry>) -> String {
    if let Some(index) = find_index_by_id(rev, &entries) {
        let data_blocks = get_data_blocks_up_to_id(rev, &entries);
        let selected_blocks = get_data_blocks_by_index(&index, &data_blocks);
        combine_data_blocks_to_text(&selected_blocks)
    } else {
        println!("No block_data blocks found for ID {}", rev);
        process::exit(1);
    }
}
/// Print header info to console
pub fn print_rev_anno_headers(headers: &Vec<RevAnnoHeader>) {
    let mut table = Table::new();

    table.add_row(Row::new(vec![
        Cell::new("rev"),
        Cell::new("offset"),
        Cell::new("length"),
    ]));
    for header in headers {
        table.add_row(Row::new(vec![
            Cell::new(&header.rev.to_string()),
            Cell::new(&header.offset.to_string()),
            Cell::new(&header.length.to_string()),
        ]));
    }
    table.printstd();

}
