use gpui::private::serde_derive::{Deserialize, Serialize};
use schemars::JsonSchema;
use settings::{Settings, SettingsSources};
use std::path::PathBuf;

#[derive(Default, Deserialize, Debug, Clone, PartialEq)]
pub struct MegaSettings {
    pub mega_url: String,
    pub fuse_url: String,
    pub mount_point: PathBuf,
    pub mega_executable: PathBuf,
    pub fuse_executable: PathBuf,
}

#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, Debug)]
pub struct MegaSettingsContent {
    /// Url to communicate with mega
    ///
    /// Default: http://localhost:8000
    pub mega_url: Option<String>,
    /// Url to communicate with fuse
    ///
    /// Default: http://localhost:2725
    pub fuse_url: Option<String>,
    /// Default mount point for fuse
    ///
    /// Default: "/" (for now)
    pub mount_point: Option<PathBuf>,
    /// Path for mega executable
    ///
    /// Default: "mega" (for now)
    pub mega_executable: Option<PathBuf>,
    /// Path for fuse executable
    ///
    /// Default: "scorpio" (for now)
    pub fuse_executable: Option<PathBuf>,
}

impl Settings for MegaSettings {
    const KEY: Option<&'static str> = Some("mega");

    type FileContent = MegaSettingsContent;

    fn load(
        sources: SettingsSources<Self::FileContent>,
        _: &mut gpui::AppContext,
    ) -> anyhow::Result<Self> {
        sources.json_merge()
    }
}
