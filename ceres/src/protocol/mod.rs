use core::fmt;
use std::{collections::HashSet, path::PathBuf, str::FromStr, sync::Arc};

use bellatrix::Bellatrix;
use callisto::sea_orm_active_enums::RefTypeEnum;
use common::{
    errors::{MegaError, ProtocolError},
    utils::ZERO_ID,
};
use import_refs::RefCommand;
use jupiter::redis::lock::RedLock;
use repo::Repo;
use tokio::sync::RwLock;

use crate::{
    api_service::state::ProtocolApiState,
    pack::{RepoHandler, import_repo::ImportRepo, monorepo::MonoRepo},
};

pub mod import_refs;
pub mod repo;
pub mod smart;

#[derive(Clone, Debug)]
pub struct PushUserInfo {
    pub username: String,
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    /// The actor username associated with the protocol operation (if available).
    pub username: Option<String>,
    /// The authenticated push user info (if available).
    pub authenticated_user: Option<PushUserInfo>,
}

#[derive(Clone, Debug)]
pub struct SmartSession {
    pub repo_path: PathBuf,
    pub service_type: ServiceType,
    pub transport_protocol: TransportProtocol,
    pub auth: AuthContext,
    pub capabilities: HashSet<Capability>,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum TransportProtocol {
    Local,
    #[default]
    Http,
    Ssh,
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
            _ => Err(MegaError::Other(format!("Invalid service name: {}", s))),
        }
    }
}

// TODO: Additional Capabilitys need to be supplemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub commands: Vec<RefCommand>,
}

impl SmartSession {
    pub fn new(
        repo_path: PathBuf,
        service_type: ServiceType,
        transport_protocol: TransportProtocol,
    ) -> Self {
        SmartSession {
            repo_path,
            service_type,
            transport_protocol,
            auth: AuthContext {
                username: None,
                authenticated_user: None,
            },
            capabilities: HashSet::new(),
        }
    }

    pub async fn repo_handler_with_commands(
        &self,
        state: &ProtocolApiState,
        commands: Vec<RefCommand>,
    ) -> Result<Arc<dyn RepoHandler>, ProtocolError> {
        let config = state.storage.config();
        let import_dir = config.monorepo.import_dir.clone();

        if self.repo_path.starts_with(import_dir.clone()) {
            let storage = state.storage.git_db_storage();
            let path_str = self.repo_path.to_str().unwrap();
            let model = storage.find_git_repo_exact_match(path_str).await.unwrap();
            let repo = if let Some(repo) = model {
                repo.into()
            } else {
                match self.service_type {
                    ServiceType::UploadPack => {
                        return Err(ProtocolError::NotFound("Repository not found.".to_owned()));
                    }
                    ServiceType::ReceivePack => {
                        let repo = Repo::new(self.repo_path.clone(), false);
                        storage.save_git_repo(repo.clone().into()).await.unwrap();
                        repo
                    }
                }
            };

            let unpack_redlock = Arc::new(RedLock::new(
                state.git_object_cache.connection.clone(),
                "git:receive-pack:lock",
                30_000, // 30s TTL
            ));
            Ok(Arc::new(ImportRepo {
                git_object_cache: state.git_object_cache.clone(),
                storage: state.storage.clone(),
                repo,
                command_list: commands,
                unpack_redlock,
            }) as Arc<dyn RepoHandler>)
        } else {
            let mut res = MonoRepo {
                git_object_cache: state.git_object_cache.clone(),
                storage: state.storage.clone(),
                path: self.repo_path.clone(),
                base_branch: "main".to_string(),
                from_hash: String::new(),
                to_hash: String::new(),
                current_commit: Arc::new(RwLock::new(None)),
                cl_link: Arc::new(RwLock::new(None)),
                bellatrix: Arc::new(Bellatrix::new(config.build.clone())),
                username: self.auth.username.clone(),
            };
            if let Some(command) = commands.iter().find(|x| x.ref_type == RefTypeEnum::Branch) {
                res.from_hash = command.old_id.clone();
                res.to_hash = command.new_id.clone();
                res.base_branch = command
                    .ref_name
                    .strip_prefix("refs/heads/")
                    .unwrap_or(command.ref_name.as_str())
                    .to_string();
            }
            Ok(Arc::new(res) as Arc<dyn RepoHandler>)
        }
    }
}

#[cfg(test)]
mod tests {}
