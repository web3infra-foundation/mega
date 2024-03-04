// use std::str::FromStr;

// use callisto::{db_enums::StorageType, raw_blob};
// use common::utils::generate_id;

// use crate::{
//     hash::SHA1,
//     internal::pack::{entry::Entry, header::EntryHeader},
// };

// impl From<Entry> for raw_blobs::Model {
//     fn from(value: Entry) -> Self {
//         raw_blobs::Model {
//             id: generate_id(),
//             sha1: value.hash.unwrap().to_plain_str(),
//             object_type: String::from_utf8_lossy(value.header.to_bytes()).to_string(),
//             storage_type: StorageType::Database,
//             data: Some(value.data),
//             local_storage_path: None,
//             remote_url: None,
//             created_at: chrono::Utc::now().naive_utc(),
//         }
//     }
// }

// impl From<raw_blobs::Model> for Entry {
//     fn from(value: raw_blobs::Model) -> Self {
//         Entry {
//             header: EntryHeader::from_string(&value.object_type),
//             offset: 0,
//             data: value.data.unwrap(),
//             hash: Some(SHA1::from_str(&value.sha1).unwrap()),
//         }
//     }
// }
