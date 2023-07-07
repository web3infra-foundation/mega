//!
//!
//!
//!
//!
pub mod http;
pub mod pack;
pub mod ssh;

use std::{
    io::Cursor,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use database::driver::{mysql::storage::MysqlStorage, ObjectStorage};

use crate::{
    errors::GitError,
    internal::{object::GitObjects, pack::decode::HashCounter},
    protocol::pack::SP,
};

use bytes::Bytes;
use entity::refs;
use sea_orm::{ActiveValue::NotSet, Set};

use crate::internal::pack::{decode::ObjDecodedMap, iterator::EntriesIter, Pack};
use common::{errors::MegaError, utils::ZERO_ID};

#[derive(Clone)]
pub struct PackProtocol {
    pub protocol: Protocol,
    pub capabilities: Vec<Capability>,
    pub path: PathBuf,
    pub storage: Arc<dyn ObjectStorage>,
    pub command_list: Vec<RefCommand>,
    // only needed in ssh protocal
    pub service_type: Option<ServiceType>,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum Protocol {
    Local,
    #[default]
    Http,
    Ssh,
    Git,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ServiceType {
    UploadPack,
    ReceivePack,
}

impl ToString for ServiceType {
    fn to_string(&self) -> String {
        match self {
            ServiceType::UploadPack => "git-upload-pack".to_owned(),
            ServiceType::ReceivePack => "git-receive-pack".to_owned(),
        }
    }
}

impl FromStr for ServiceType {
    type Err = MegaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "git-upload-pack" => Ok(ServiceType::UploadPack),
            "git-receive-pack" => Ok(ServiceType::ReceivePack),
            _ => Err(MegaError {
                error: anyhow::anyhow!("Invalid service name: {}", s).into(),
                code: 400,
            }),
        }
    }
}

// TODO: Additional Capabilitys need to be supplemented.
#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    MultiAck,
    MultiAckDetailed,
    NoDone,
    SideBand,
    SideBand64k,
    ReportStatus,
    ReportStatusv2,
    OfsDelta,
    DeepenSince,
    DeepenNot,
}

impl FromStr for Capability {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "report-status" => Ok(Capability::ReportStatus),
            "report-status-v2" => Ok(Capability::ReportStatusv2),
            "side-band" => Ok(Capability::SideBand),
            "side-band-64k" => Ok(Capability::SideBand64k),
            "ofs-delta" => Ok(Capability::OfsDelta),
            "multi_ack" => Ok(Capability::MultiAck),
            "multi_ack_detailed" => Ok(Capability::MultiAckDetailed),
            "no-done" => Ok(Capability::NoDone),
            "deepen-since" => Ok(Capability::DeepenSince),
            "deepen-not" => Ok(Capability::DeepenNot),
            _ => Err(()),
        }
    }
}

pub enum SideBind {
    // sideband 1 will contain packfile data,
    PackfileData,
    // sideband 2 will be used for progress information that the client will generally print to stderr and
    ProgressInfo,
    // sideband 3 is used for error information.
    Error,
}

impl SideBind {
    pub fn value(&self) -> u8 {
        match self {
            Self::PackfileData => b'\x01',
            Self::ProgressInfo => b'\x02',
            Self::Error => b'\x03',
        }
    }
}
pub struct RefUpdateRequet {
    pub comand_list: Vec<RefCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefCommand {
    pub ref_name: String,
    pub old_id: String,
    pub new_id: String,
    pub status: String,
    pub error_msg: String,
    pub command_type: CommandType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Create,
    Delete,
    Update,
}

impl RefCommand {
    const OK_STATUS: &str = "ok";

    const FAILED_STATUS: &str = "ng";

    pub fn new(old_id: String, new_id: String, ref_name: String) -> Self {
        let command_type = if ZERO_ID == old_id {
            CommandType::Create
        } else if ZERO_ID == new_id {
            CommandType::Delete
        } else {
            CommandType::Update
        };
        RefCommand {
            ref_name,
            old_id,
            new_id,
            status: RefCommand::OK_STATUS.to_owned(),
            error_msg: "".to_owned(),
            command_type,
        }
    }

    pub async fn unpack(
        &mut self,
        pack_file: &mut Bytes,
        // storage: &T,
    ) -> Result<ObjDecodedMap, anyhow::Error> {
        let obj: Result<ObjDecodedMap, GitError> = {
            let mut object_map = ObjDecodedMap::default();
            let count_hash: bool = true;
            let curosr_pack = Cursor::new(pack_file);
            let mut reader = HashCounter::new(curosr_pack, count_hash);
            // Read the header of the pack file
            let mut pack = Pack::check_header(&mut reader)?;

            let mut iterator = EntriesIter::new(&mut reader, pack.number_of_objects() as u32);
            for _ in 0..pack.number_of_objects() {
                let obj = iterator.next_git_obj().await?;
                println!("{}", obj);
                match obj {
                    GitObjects::COMMIT(a) => {
                        object_map.commits.push(a);
                    }
                    GitObjects::TREE(a) => {
                        object_map.trees.push(a);
                    }
                    GitObjects::BLOB(a) => {
                        object_map.blobs.push(a);
                    }
                    GitObjects::TAG(a) => {
                        object_map.tags.push(a);
                    }
                }
            }
            drop(iterator);

            let _hash = reader.final_hash();
            pack.signature = _hash;
            //pack.signature = Hash::new_from_bytes(&id[..]);

            // pack.signature = read_tail_hash(&mut reader);
            // assert_eq!(_hash, pack.signature);

            Ok(object_map)
        };
        match obj {
            Ok(object_map) => {
                self.status = RefCommand::OK_STATUS.to_owned();
                Ok(object_map)
            }
            Err(err) => {
                self.status = RefCommand::FAILED_STATUS.to_owned();
                self.error_msg = err.to_string();
                Err(err.into())
            }
        }

        // match Pack::decode(&mut Cursor::new(&pack_file)).await {
        //     Ok(decoded_pack) => {
        //         let inner = &mut Cursor::new(&pack_file);
        //         let mut inter = EntriesIter::new(
        //             BufReader::with_capacity(4096, inner),
        //             decoded_pack.number_of_objects() as u32,
        //         );

        //         for _ in 0..decoded_pack.number_of_objects() {
        //             let obj = inter.next_obj().await?;
        //             println!("{}", obj);
        //             let raw = obj.get_raw().to_vec();
        //             //let rrr: Arc<Mutex<dyn Any>> = Arc::new(Mutex::new(_blob));
        //             match obj.get_type() {
        //                 ObjectType::Commit => object_map.commits.push(Commit::new_from_data(raw)),
        //                 ObjectType::Tree => object_map.trees.push(Tree::new_from_data(raw)),
        //                 ObjectType::Blob => object_map.blobs.push(Blob::new_from_data(raw)),
        //                 ObjectType::Tag => object_map.tags.push(Tag::new_from_data(raw)),
        //                 _ => todo!(),
        //             }
        //         }

        //         self.status = RefCommand::OK_STATUS.to_owned();
        //         Ok(object_map)
        //     }
        //     Err(err) => {
        //         self.status = RefCommand::FAILED_STATUS.to_owned();
        //         self.error_msg = err.to_string();
        //         Err(err.into())
        //     }
    }

    pub fn get_status(&self) -> String {
        if RefCommand::OK_STATUS == self.status {
            format!("{}{}{}", self.status, SP, self.ref_name,)
        } else {
            format!(
                "{}{}{}{}{}",
                self.status,
                SP,
                self.ref_name,
                SP,
                self.error_msg.clone()
            )
        }
    }

    pub fn failed(&mut self, msg: String) {
        self.status = RefCommand::FAILED_STATUS.to_owned();
        self.error_msg = msg;
    }

    pub fn convert_to_model(&self, path: &str) -> refs::ActiveModel {
        refs::ActiveModel {
            id: NotSet,
            ref_git_id: Set(self.new_id.to_owned()),
            ref_name: Set(self.ref_name.to_string()),
            repo_path: Set(path.to_owned()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
}
impl PackProtocol {
    pub fn new(path: PathBuf, storage: Arc<dyn ObjectStorage>, protocol: Protocol) -> Self {
        PackProtocol {
            protocol,
            capabilities: Vec::new(),
            path,
            storage,
            command_list: Vec::new(),
            service_type: None,
        }
    }

    pub fn mock() -> Self {
        PackProtocol {
            protocol: Protocol::default(),
            capabilities: Vec::new(),
            path: PathBuf::new(),
            storage: Arc::new(MysqlStorage::default()),
            command_list: Vec::new(),
            service_type: None,
        }
    }
}

#[cfg(test)]
mod tests {}
