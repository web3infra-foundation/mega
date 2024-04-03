//!
//!
//!
//!
//!
use std::{env, path::PathBuf, str::FromStr};

use callisto::db_enums::RefType;
use common::{
    errors::MegaError,
    utils::{generate_id, ZERO_ID},
};
use jupiter::context::Context;
use venus::{internal::pack::reference::RefCommand, repo::Repo};

use crate::pack::{handler::PackHandler, import_repo::ImportRepo, monorepo::MonoRepo};

pub mod smart;

#[derive(Clone)]
pub struct SmartProtocol {
    pub transport_protocol: TransportProtocol,
    pub capabilities: Vec<Capability>,
    pub path: PathBuf,
    pub command_list: Vec<RefCommand>,
    // only needed in ssh protocal
    pub service_type: ServiceType,
    pub context: Context,
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
pub struct RefUpdateRequest {
    pub command_list: Vec<RefCommand>,
}

impl SmartProtocol {
    pub fn new(path: PathBuf, context: Context, transport_protocol: TransportProtocol) -> Self {
        SmartProtocol {
            transport_protocol,
            capabilities: Vec::new(),
            path,
            command_list: Vec::new(),
            service_type: ServiceType::ReceivePack,
            context,
        }
    }

    pub fn mock() -> Self {
        let context = Context::mock();
        SmartProtocol {
            transport_protocol: TransportProtocol::default(),
            capabilities: Vec::new(),
            path: PathBuf::new(),
            command_list: Vec::new(),
            service_type: ServiceType::ReceivePack,
            context,
        }
    }

    pub async fn pack_handler(&self) -> Box<dyn PackHandler> {
        let import_dir = PathBuf::from(env::var("MEGA_IMPORT_DIRS").unwrap());
        if self.path.starts_with(import_dir.clone()) && self.path != import_dir {
            let storage = self.context.services.git_db_storage.clone();

            let path_str = self.path.to_str().unwrap();
            let model = storage.find_git_repo(path_str).await.unwrap();
            let repo = if let Some(repo) = model {
                repo.into()
            } else {
                let repo_name = self.path.file_name().unwrap().to_str().unwrap().to_owned();
                let repo = Repo {
                    repo_id: generate_id(),
                    repo_path: self.path.to_str().unwrap().to_owned(),
                    repo_name,
                };
                storage.save_git_repo(repo.clone()).await.unwrap();
                repo
            };
            Box::new(ImportRepo {
                context: self.context.clone(),
                repo,
            })
        } else {
            let mut res = Box::new(MonoRepo {
                context: self.context.clone(),
                path: self.path.clone(),
                from_hash: None,
                to_hash: None,
            });
            if let Some(command) = self
                .command_list
                .iter()
                .find(|x| x.ref_type == RefType::Branch)
            {
                res.from_hash = Some(command.old_id.clone());
                res.to_hash = Some(command.new_id.clone());
            }
            res
        }
    }
}

#[cfg(test)]
mod tests {}
