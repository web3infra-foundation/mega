//!
//!
//!
//!
//!
pub mod http;
pub mod pack;
pub mod ssh;

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use storage::driver::{database::mysql_storage::MysqlStorage, database::storage::ObjectStorage};

use crate::protocol::pack::SP;

use common::{errors::MegaError, utils::ZERO_ID};
use entity::{mr_info, refs};
use sea_orm::{ActiveValue::NotSet, Set};

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
    P2p,
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
    pub refs_type: RefsType,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum RefsType {
    #[default]
    Branch,
    Tag,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Create,
    Delete,
    Update,
}

impl RefCommand {
    const OK_STATUS: &'static str = "ok";

    const FAILED_STATUS: &'static str = "ng";

    pub fn new(old_id: String, new_id: String, ref_name: String) -> Self {
        let command_type = if ZERO_ID == old_id {
            CommandType::Create
        } else if ZERO_ID == new_id {
            CommandType::Delete
        } else {
            CommandType::Update
        };
        RefCommand {
            ref_name: ref_name.clone(),
            old_id,
            new_id,
            status: RefCommand::OK_STATUS.to_owned(),
            error_msg: "".to_owned(),
            command_type,
            refs_type: if ref_name.starts_with("refs/tags") {
                RefsType::Tag
            } else {
                RefsType::Branch
            },
        }
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

    pub async fn update_refs(&self, storage: Arc<dyn ObjectStorage>, path: &Path) {
        match self.command_type {
            CommandType::Create => {
                storage
                    .save_refs(vec![self.convert_to_model(path.to_str().unwrap())])
                    .await
                    .unwrap();
            }
            CommandType::Delete => storage.delete_refs(self.old_id.clone(), path).await,
            CommandType::Update => {
                storage
                    .update_refs(self.old_id.clone(), self.new_id.clone(), path)
                    .await;
            }
        }
    }
}

pub fn new_mr_info(mr_id: i64) -> mr_info::ActiveModel {
    mr_info::ActiveModel {
        id: NotSet,
        mr_id: Set(mr_id),
        mr_msg: Set("repo initialize".to_owned()),
        mr_date: Set(chrono::Utc::now().naive_utc()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
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
