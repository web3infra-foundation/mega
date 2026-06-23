//! Scheme C2: ensure every cell resolves a target platform instead of `<unspecified>`.
//!
//! Orion C1 removed CLI `--target-platforms`; platform must come from `.buckconfig`.
//! Repos with an incomplete `target_platform_detector_spec` (e.g. only `root//...`)
//! leave `buckal//tool:manifest_parse` unconfigured and fail analysis with
//! `manifest_parse (<unspecified>)`.

use std::process::Command;

/// Global default when a target has no per-target `default_target_platform`.
pub const DEFAULT_TARGET_PLATFORM: &str = "prelude//platforms:default";

/// Maps all monorepo cells to the shared default platform.
pub const TARGET_PLATFORM_DETECTOR_SPEC: &str = "\
target:root//...->prelude//platforms:default \
target:prelude//...->prelude//platforms:default \
target:toolchains//...->prelude//platforms:default \
target:buckal//...->prelude//platforms:default";

/// Append Buck2 `--config` overrides so platform resolution works even when the
/// checked-in `.buckconfig` is incomplete (read-only on Antares mounts).
pub fn append_platform_config(command: &mut Command) {
    command
        .arg("--config")
        .arg(format!(
            "build.default_target_platforms={DEFAULT_TARGET_PLATFORM}"
        ))
        .arg("--config")
        .arg(format!(
            "parser.target_platform_detector_spec={TARGET_PLATFORM_DETECTOR_SPEC}"
        ));
}

/// Config key/value pairs for async command builders (e.g. `tokio::process::Command`).
pub fn platform_config_flags() -> [String; 4] {
    [
        "--config".to_owned(),
        format!("build.default_target_platforms={DEFAULT_TARGET_PLATFORM}"),
        "--config".to_owned(),
        format!("parser.target_platform_detector_spec={TARGET_PLATFORM_DETECTOR_SPEC}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_spec_covers_buckal_cell() {
        assert!(TARGET_PLATFORM_DETECTOR_SPEC.contains("target:buckal//..."));
    }
}
