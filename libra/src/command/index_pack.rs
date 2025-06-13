use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use byteorder::{BigEndian, WriteBytesExt};
use clap::Parser;
use sha1::{Digest, Sha1};

use mercury::errors::GitError;
use mercury::internal::pack::Pack;

#[derive(Parser, Debug)]
pub struct IndexPackArgs {
    /// Pack file path
    pub pack_file: String,
    /// output index file path.
    /// Without this option the name of pack index file is constructed from
    /// the name of packed archive file by replacing `.pack` with `.idx`
    #[clap(short = 'o', required = false)]
    pub index_file: Option<String>, // Option is must, or clap will require it

    /// This is intended to be used by the test suite only.
    /// It allows to force the version for the generated pack index
    #[clap(long, required = false)]
    pub index_version: Option<u8>,
}

pub fn execute(args: IndexPackArgs) {
    let pack_file = args.pack_file;
    let index_file = args.index_file.unwrap_or_else(|| {
        if !pack_file.ends_with(".pack") {
            eprintln!("fatal: pack-file does not end with '.pack'");
            return String::new();
        }
        pack_file.replace(".pack", ".idx")
    });
    if index_file.is_empty() {
        return;
    }
    if index_file == pack_file {
        eprintln!("fatal: pack-file and index-file are the same file");
        return;
    }

    if let Some(version) = args.index_version {
        match version {
            1 => build_index_v1(&pack_file, &index_file).unwrap(),
            2 => println!("support later"),
            _ => eprintln!("fatal: unsupported index version"),
        }
    } else {
        // default version = 1
        build_index_v1(&pack_file, &index_file).unwrap();
    }
}

/// Build index file for pack file, version 1
/// [pack-format](https://git-scm.com/docs/pack-format)
pub fn build_index_v1(pack_file: &str, index_file: &str) -> Result<(), GitError> {
    let pack_path = PathBuf::from(pack_file);
    let tmp_path = pack_path.parent().unwrap();
    let pack_file = std::fs::File::open(pack_file)?;
    let mut pack_reader = std::io::BufReader::new(pack_file);
    let obj_map = Arc::new(Mutex::new(BTreeMap::new())); // sorted by hash
    let obj_map_c = obj_map.clone();
    let mut pack = Pack::new(
        Some(8),
        Some(1024 * 1024 * 1024),
        Some(tmp_path.to_path_buf()),
        true,
    );
    pack.decode(&mut pack_reader, move |entry, offset| {
        obj_map_c.lock().unwrap().insert(entry.hash, offset);
    })?;

    let mut index_hash = Sha1::new();
    let mut index_file = std::fs::File::create(index_file)?;
    // fan-out table
    // The header consists of 256 4-byte network byte order integers.
    // N-th entry of this table records the number of objects in the corresponding pack,
    // the first byte of whose object name is less than or equal to N.
    // This is called the first-level fan-out table.
    let mut i: u8 = 0;
    let mut cnt: u32 = 0;
    let mut fan_out = Vec::with_capacity(256 * 4);
    let obj_map = Arc::try_unwrap(obj_map).unwrap().into_inner().unwrap();
    for (hash, _) in obj_map.iter() {
        // sorted
        let first_byte = hash.0[0];
        while first_byte > i {
            // `while` rather than `if` to fill the gap, e.g. 0, 1, 2, 2, 2, 6
            fan_out.write_u32::<BigEndian>(cnt)?;
            i += 1;
        }
        cnt += 1;
    }
    // fill the rest
    loop {
        fan_out.write_u32::<BigEndian>(cnt)?;
        if i == 255 {
            break;
        }
        i += 1;
    }
    index_hash.update(&fan_out);
    index_file.write_all(&fan_out)?;

    // 4-byte network byte order integer, recording where the
    // object is stored in the pack-file as the offset from the beginning.
    // one object name of the appropriate size (20 bytes).
    for (hash, offset) in obj_map {
        let mut buf = Vec::with_capacity(24);
        buf.write_u32::<BigEndian>(offset as u32)?;
        buf.write_all(&hash.0)?;

        index_hash.update(&buf);
        index_file.write_all(&buf)?;
    }

    index_hash.update(pack.signature.0);
    // A copy of the pack checksum at the end of the corresponding pack-file.
    index_file.write_all(&pack.signature.0)?;
    let index_hash: [u8; 20] = index_hash.finalize().into();
    // Index checksum of all of the above.
    index_file.write_all(&index_hash)?;

    tracing::debug!("Index file is written to {:?}", index_file);
    Ok(())
}
