use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum Status<Path: Clone> {
    Modified(Path),
    Added(Path),
    Removed(Path),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn from_abs(abs_path: &str, base: &str) -> Option<Self> {
        let opt = abs_path
            .strip_prefix(base)
            .map(|s| s.trim_start_matches("/"));
        match opt {
            Some(s) => Some(Self(s.to_owned())),
            None => None,
        }
    }
}

impl<Path: Clone> Status<Path> {
    pub fn path(&self) -> Path {
        match self {
            Self::Added(p) => p.clone(),
            Self::Modified(p) => p.clone(),
            Self::Removed(p) => p.clone(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct BuildInfo {
    pub buck_hash: String,
    pub buckconfig_hash: String,
    pub args: Option<Vec<String>>,
    pub changes: Vec<Status<ProjectRelativePath>>,
}

#[derive(Serialize, Debug)]
pub struct OrionBuildRequest {
    pub cl_link: String,
    pub repo: String,
    pub cl: i64,
    pub task_name: Option<String>,
    pub template: Option<String>,
    pub builds: Vec<BuildInfo>,
}

#[derive(Clone)]
pub(crate) struct OrionClient {
    base_url: String,
    client: reqwest::Client,
}

impl OrionClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn trigger_build(&self, req: OrionBuildRequest) -> anyhow::Result<()> {
        let url = format!("{}/task", self.base_url);
        tracing::info!("Try to trigger build with params:{:?}", req);
        let res = self.client.post(&url).json(&req).send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            tracing::error!("Failed to trigger build: {}", res.status());
            Err(anyhow::anyhow!("Failed to trigger build: {}", res.status()))
        }
    }
}
