use gpui::private::serde::{Deserialize, Serialize};
use gpui::Pixels;
use schemars::JsonSchema;
use settings::{Settings, SettingsSources};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MegaPanelDockPosition {
    Left,
    Right,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct MegaPanelSettings {
    pub button: bool,
    pub default_width: Pixels,
    pub dock: MegaPanelDockPosition,
}

#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, Debug)]
pub struct MegaPanelSettingsContent {
    /// Whether to show the mega panel button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Customize default width (in pixels) taken by mega panel
    ///
    /// Default: 240
    pub default_width: Option<f32>,
    /// The position of mega panel
    ///
    /// Default: left
    pub dock: Option<MegaPanelDockPosition>,
}

impl Settings for MegaPanelSettings {
    const KEY: Option<&'static str> = Some("mega_panel");

    type FileContent = MegaPanelSettingsContent;

    fn load(
        sources: SettingsSources<Self::FileContent>,
        _: &mut gpui::AppContext,
    ) -> anyhow::Result<Self> {
        sources.json_merge()
    }
}
