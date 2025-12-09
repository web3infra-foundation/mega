mod fuse;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::dicfuse::Dicfuse;
use crate::util::config;

/// Global paths used by Antares to place layers and state.
#[derive(Debug, Clone)]
pub struct AntaresPaths {
    /// Root directory to place per-job upper layers.
    pub upper_root: PathBuf,
    /// Root directory to place per-job CL layers when requested.
    pub cl_root: PathBuf,
    /// Base directory for mountpoints returned to callers.
    pub mount_root: PathBuf,
    /// Path to persist mount state as TOML.
    pub state_file: PathBuf,
}

impl AntaresPaths {
    pub fn new(
        upper_root: PathBuf,
        cl_root: PathBuf,
        mount_root: PathBuf,
        state_file: PathBuf,
    ) -> Self {
        Self {
            upper_root,
            cl_root,
            mount_root,
            state_file,
        }
    }

    /// Build paths using global config defaults.
    pub fn from_global_config() -> Self {
        Self {
            upper_root: PathBuf::from(config::antares_upper_root()),
            cl_root: PathBuf::from(config::antares_cl_root()),
            mount_root: PathBuf::from(config::antares_mount_root()),
            state_file: PathBuf::from(config::antares_state_file()),
        }
    }
}

/// Persisted config for a mounted Antares job instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntaresConfig {
    pub job_id: String,
    pub mountpoint: PathBuf,
    pub upper_id: String,
    pub upper_dir: PathBuf,
    pub cl_dir: Option<PathBuf>,
    pub cl_id: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AntaresState {
    mounts: Vec<AntaresConfig>,
}

/// Manager responsible for creating and tracking Antares overlay instances.
/// This scaffold currently wires directory creation and bookkeeping; the unionfs
/// integration will be added once the layer stack is finalized.
pub struct AntaresManager {
    dic: Arc<Dicfuse>,
    paths: AntaresPaths,
    instances: Arc<Mutex<HashMap<String, AntaresConfig>>>,
}

impl AntaresManager {
    /// Build an independent Antares manager with its own Dicfuse instance.
    pub async fn new(paths: AntaresPaths) -> Self {
        let dic = Arc::new(Dicfuse::new().await);
        let instances = Self::load_state(&paths.state_file).unwrap_or_default();
        Self {
            dic,
            paths,
            instances: Arc::new(Mutex::new(instances)),
        }
    }

    /// Create directories and register a job instance. UnionFS wiring is added later.
    pub async fn mount_job(
        &self,
        job_id: &str,
        cl_name: Option<&str>,
    ) -> std::io::Result<AntaresConfig> {
        // Prepare per-job paths
        let upper_id = Uuid::new_v4().to_string();
        let upper_dir = self.paths.upper_root.join(&upper_id);
        let (cl_id, cl_dir) = match cl_name {
            Some(_) => {
                let id = Uuid::new_v4().to_string();
                (Some(id.clone()), Some(self.paths.cl_root.join(id)))
            }
            None => (None, None),
        };
        let mountpoint = self.paths.mount_root.join(job_id);

        std::fs::create_dir_all(&upper_dir)?;
        if let Some(cl) = &cl_dir {
            std::fs::create_dir_all(cl)?;
        }
        std::fs::create_dir_all(&mountpoint)?;

        let instance = AntaresConfig {
            job_id: job_id.to_string(),
            mountpoint,
            upper_id,
            upper_dir,
            cl_dir,
            cl_id,
        };

        self.instances
            .lock()
            .await
            .insert(job_id.to_string(), instance.clone());

        self.persist_state().await?;

        Ok(instance)
    }

    /// Remove bookkeeping for a job. FS teardown will be added later.
    pub async fn umount_job(&self, job_id: &str) -> std::io::Result<Option<AntaresConfig>> {
        let removed = self.instances.lock().await.remove(job_id);
        self.persist_state().await?;
        Ok(removed)
    }

    /// List all tracked instances.
    pub async fn list(&self) -> Vec<AntaresConfig> {
        self.instances.lock().await.values().cloned().collect()
    }

    /// Access the underlying Dicfuse instance (read-only tree layer).
    pub fn dicfuse(&self) -> Arc<Dicfuse> {
        self.dic.clone()
    }

    fn load_state(path: &Path) -> std::io::Result<HashMap<String, AntaresConfig>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(path)?;
        let state: AntaresState = toml::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("parse state: {e}"))
        })?;
        let mut map = HashMap::new();
        for m in state.mounts {
            map.insert(m.job_id.clone(), m);
        }
        Ok(map)
    }

    async fn persist_state(&self) -> std::io::Result<()> {
        let mounts: Vec<AntaresConfig> = self.instances.lock().await.values().cloned().collect();
        let state = AntaresState { mounts };
        let data = toml::to_string_pretty(&state).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("encode state: {e}"))
        })?;
        if let Some(parent) = self.paths.state_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = File::create(&self.paths.state_file)?;
        f.write_all(data.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn mount_and_list_registers_instance() {
        let root = tempdir().unwrap();
        let upper = root.path().join("upper");
        let cl = root.path().join("cl");
        let mnt = root.path().join("mnt");
        let state = root.path().join("state.toml");

        let paths = AntaresPaths::new(upper, cl, mnt.clone(), state);
        let manager = AntaresManager::new(paths).await;

        let instance = manager.mount_job("job1", Some("cl1")).await.unwrap();
        assert_eq!(instance.job_id, "job1");
        assert!(instance.mountpoint.starts_with(&mnt));

        // state file should be written with the mount record
        let state_content = std::fs::read_to_string(root.path().join("state.toml")).unwrap();
        let parsed: AntaresState = toml::from_str(&state_content).unwrap();
        assert_eq!(parsed.mounts.len(), 1);
        assert_eq!(parsed.mounts[0].job_id, "job1");

        let listed = manager.list().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].job_id, "job1");

        let removed = manager.umount_job("job1").await.unwrap();
        assert!(removed.is_some());
        assert!(manager.list().await.is_empty());
    }
}
