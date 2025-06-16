use core::fmt;
use std::{path::PathBuf, str::FromStr, sync::Arc};

use callisto::sea_orm_active_enums::RefTypeEnum;
use common::{
    errors::{MegaError, ProtocolError},
    utils::ZERO_ID,
};
use import_refs::RefCommand;
use jupiter::context::Storage;
use repo::Repo;

use crate::pack::{import_repo::ImportRepo, monorepo::MonoRepo, PackHandler};

pub mod import_refs;
pub mod mr;
pub mod repo;
pub mod smart;

#[derive(Clone)]
pub struct SmartProtocol {
    pub transport_protocol: TransportProtocol,
    pub capabilities: Vec<Capability>,
    pub path: PathBuf,
    pub command_list: Vec<RefCommand>,
    pub service_type: Option<ServiceType>,
    pub storage: Storage,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum TransportProtocol {
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

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceType::UploadPack => write!(f, "git-upload-pack"),
            ServiceType::ReceivePack => write!(f, "git-receive-pack"),
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
pub struct RefUpdateRequest {
    pub command_list: Vec<RefCommand>,
}

impl SmartProtocol {
    pub fn new(path: PathBuf, storage: Storage, transport_protocol: TransportProtocol) -> Self {
        SmartProtocol {
            transport_protocol,
            capabilities: Vec::new(),
            path,
            command_list: Vec::new(),
            service_type: None,
            storage,
        }
    }

    pub fn mock() -> Self {
        let context = Storage::mock();
        SmartProtocol {
            transport_protocol: TransportProtocol::default(),
            capabilities: Vec::new(),
            path: PathBuf::new(),
            command_list: Vec::new(),
            service_type: None,
            storage: context,
        }
    }

    pub async fn pack_handler(&self) -> Result<Arc<dyn PackHandler>, ProtocolError> {
        let import_dir = self.storage.config().monorepo.import_dir.clone();
        if self.path.starts_with(import_dir.clone()) {
            let storage = self.storage.services.git_db_storage.clone();
            let path_str = self.path.to_str().unwrap();
            let model = storage.find_git_repo_exact_match(path_str).await.unwrap();
            let repo = if let Some(repo) = model {
                repo.into()
            } else {
                match self.service_type.unwrap() {
                    ServiceType::UploadPack => {
                        return Err(ProtocolError::NotFound("Repository not found.".to_owned()))
                    }
                    ServiceType::ReceivePack => {
                        let repo = Repo::new(self.path.clone(), false);
                        storage.save_git_repo(repo.clone().into()).await.unwrap();
                        repo
                    }
                }
            };
            Ok(Arc::new(ImportRepo {
                storage: self.storage.clone(),
                repo,
                command_list: self.command_list.clone(),
            }))
        } else {
            let mut res = MonoRepo {
                storage: self.storage.clone(),
                path: self.path.clone(),
                from_hash: String::new(),
                to_hash: String::new(),
            };
            if let Some(command) = self
                .command_list
                .iter()
                .find(|x| x.ref_type == RefTypeEnum::Branch)
            {
                res.from_hash = command.old_id.clone();
                res.to_hash = command.new_id.clone();
            }
            Ok(Arc::new(res))
        }
    }
}

#[cfg(test)]
mod tests {}
