/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::{
    collections::HashMap,
    fmt::{self, Display},
    str::FromStr,
};

use api_model::buck2::ws::WSTargetBuildStatusUpdate;
use serde::{Deserialize, Serialize};

pub const EVENT_LOG_FILE: &str = "event.jsonl";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetBuildStatusUpdate {
    /// Logical action identifier
    pub action_id: LogicalActionId,
    /// Previous execution status
    pub old_status: ExecutionStatus,
    /// New execution status
    pub new_status: ExecutionStatus,
    // Todo: Add timestamp and error_message
}

impl From<TargetBuildStatusUpdate> for WSTargetBuildStatusUpdate {
    fn from(value: TargetBuildStatusUpdate) -> Self {
        Self {
            configured_target_package: value.action_id.configured_target.target.package,
            configured_target_name: value.action_id.configured_target.target.name,
            configured_target_configuration: value.action_id.configured_target.configuration,
            category: value.action_id.category,
            identifier: value.action_id.identifier,
            action: value.action_id.action.to_string(),
            old_status: value.old_status.to_string(),
            new_status: value.new_status.to_string(),
        }
    }
}

impl From<WSTargetBuildStatusUpdate> for TargetBuildStatusUpdate {
    fn from(value: WSTargetBuildStatusUpdate) -> Self {
        Self {
            action_id: LogicalActionId {
                configured_target: ConfiguredTargetId {
                    target: TargetBuildId {
                        package: value.configured_target_package,
                        name: value.configured_target_name,
                    },
                    configuration: value.configured_target_configuration,
                },
                category: value.category,
                identifier: value.identifier,
                action: ActionKind::from_str(&value.action).unwrap_or(ActionKind::Other),
            },
            old_status: ExecutionStatus::from_str(&value.old_status)
                .unwrap_or(ExecutionStatus::Pending),
            new_status: ExecutionStatus::from_str(&value.new_status)
                .unwrap_or(ExecutionStatus::Pending),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    // Cache
}

impl Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Succeeded => "succeeded",
            ExecutionStatus::Failed => "failed",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ExecutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ExecutionStatus::Pending),
            "running" => Ok(ExecutionStatus::Running),
            "succeeded" | "success" => Ok(ExecutionStatus::Succeeded),
            "failed" | "failure" => Ok(ExecutionStatus::Failed),
            _ => Err(format!("Unknown execution status: {}", s)),
        }
    }
}

/// Hierarchical yet flat data model for Buck2 build events.
///
/// Conceptual hierarchy:
/// ```
/// Target (package + name)
///     └── ConfiguredTarget (+ configuration)
///             └── LogicalAction (kind + category + identifier)
/// ```
///
/// In-memory storage is flat and ID-indexed using `HashMap`s for O(1) access:
/// - `targets: HashMap<TargetBuildId, TargetState>`
/// - `configured_targets: HashMap<ConfiguredTargetId, ConfiguredTargetState>`
/// - `actions: HashMap<LogicalActionId, LogicalActionState>`
///
/// # Layer Definitions
/// - **Target:** Top-level build unit (`package + name`) corresponding to `TargetLabel`.
/// - **ConfiguredTarget:** Target with a specific configuration (platform, optimization, etc.).
/// - **LogicalAction:** Smallest observable unit. Aggregates all events sharing the same:
///   `(package, name, configuration, kind, category, identifier)`
///
/// # Example Mapping
/// ```text
/// Target: root//third-party/rust/crates/futures-executor/0.3.31 + futures-executor
/// └── ConfiguredTarget: prelude//platforms:default#462c4a1659836f33
///     └── LogicalAction: Write + write + LPPM/futures_executor-metadata-fast-diag.args
/// ```
///
/// # Notes
/// - Flat structure avoids nested Vecs and enables scalable updates.
/// - Designed for single-threaded state machine; parsing and I/O can be concurrent.
#[derive(Debug, Default)]
pub struct BuildState {
    /// All targets across the entire build.
    // pub targets: HashMap<TargetBuildId, TargetState>,
    /// All configured targets across the entire build.
    // pub configured_targets: HashMap<ConfiguredTargetId, ConfiguredTargetState>,

    /// All logical actions across the entire build.
    pub actions: HashMap<LogicalActionId, LogicalActionState>,
}

impl BuildState {
    /// Handle a buck2 event.
    /// If a logical action status changes, return StatusUpdate.
    pub fn handle_event(&mut self, event: &Event) -> Option<TargetBuildStatusUpdate> {
        // 1. Only process action_execution related events
        let action_execution = event.action_execution()?;

        // 2. Generate action_id
        let action_id = LogicalActionId::from_event(event)?;

        // 3. Get new_status
        let new_status = event.execution_status()?;

        // 4. Insert or get action
        let action = self.actions.entry(action_id.clone()).or_insert_with(|| {
            LogicalActionState::new(&action_id, action_execution.action_kind.clone())
        });

        // 5. Skip broadcast if status unchanged
        if action.status == new_status {
            return None;
        }

        // 6. Update status
        let old_status = action.status;
        action.status = new_status;

        // 7. Build StatusUpdate
        Some(TargetBuildStatusUpdate {
            action_id,
            old_status,
            new_status,
        })
    }
}

/// Represents a Buck2 target (package + name).
#[derive(Debug)]
pub struct TargetState {
    /// Unique target identity (package + name).
    pub id: TargetBuildId,

    /// A single target may be built multiple times under different configurations.
    ///
    /// Stores IDs only to avoid deep nesting and large memory footprint.
    pub configured_target_ids: Vec<ConfiguredTargetId>,
}

impl TargetState {
    pub fn new(target_build_id: &TargetBuildId) -> Self {
        Self {
            id: target_build_id.to_owned(),
            configured_target_ids: Vec::new(),
        }
    }
}

/// Represents a configured target (target + configuration).
#[derive(Debug)]
pub struct ConfiguredTargetState {
    /// Uniquely identifies a configured target: (target + configuration).
    pub id: ConfiguredTargetId,

    /// All logical actions executed under this configured target.
    ///
    /// Stores IDs only to keep the structure lightweight.
    pub action_ids: Vec<LogicalActionId>,
}

impl ConfiguredTargetState {
    pub fn new(configured_target_id: &ConfiguredTargetId) -> Self {
        Self {
            id: configured_target_id.to_owned(),
            action_ids: Vec::new(),
        }
    }
}

/// Represents the runtime state of a logical action.
#[derive(Debug)]
pub struct LogicalActionState {
    /// Unique identity for this logical action.
    pub id: LogicalActionId,

    /// The type of action being executed (e.g., Write, Compile, Copy).
    /// Determines how the action is processed and what resources it requires.
    pub kind: ActionKind,

    /// Current execution status (pending, running, completed, failed).
    /// Updated as events are processed.
    pub status: ExecutionStatus,
}

impl LogicalActionState {
    pub fn new(logical_action_id: &LogicalActionId, kind: ActionKind) -> Self {
        Self {
            id: logical_action_id.to_owned(),
            kind,
            status: ExecutionStatus::Pending,
        }
    }

    pub fn with_status(id: LogicalActionId, kind: ActionKind, status: ExecutionStatus) -> Self {
        Self { id, kind, status }
    }
}

/// Uniquely identifies a Buck2 target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetBuildId {
    /// Buck2 package path
    /// (e.g., "root//third-party/rust/crates/flate2/1.1.2").
    pub package: String,

    /// Target name inside the package (e.g., "flate2").
    pub name: String,
}

impl TargetBuildId {
    pub fn new(package: &str, name: &str) -> Self {
        Self {
            package: package.to_owned(),
            name: name.to_owned(),
        }
    }
}

/// Uniquely identifies a configured target (target + configuration).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfiguredTargetId {
    /// The base target being configured.
    pub target: TargetBuildId,

    /// Configuration full name
    /// (e.g., "prelude//platforms:default#b42aeba648b8c415").
    /// Encodes platform, optimization level, and other build parameters.
    pub configuration: String,
}

impl ConfiguredTargetId {
    pub fn new(target_id: &TargetBuildId, configuration: &str) -> Self {
        Self {
            target: target_id.to_owned(),
            configuration: configuration.to_owned(),
        }
    }
}

/// Uniquely identifies a logical action across the entire build.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalActionId {
    /// The configured target that owns this action.
    pub configured_target: ConfiguredTargetId,

    /// Action category for analytics and UI grouping
    /// (e.g., "cxx_compile", "write_json").
    pub category: String,

    /// Unique identifier within the category
    /// (e.g., XIPL/build_script_build-link-diag.args).
    pub identifier: String,

    /// Low-level execution type (e.g., Write, Copy, Run).
    pub action: ActionKind,
}

impl LogicalActionId {
    pub fn new(
        configured_target: &ConfiguredTargetId,
        category: &str,
        identifier: &str,
        action: ActionKind,
    ) -> Self {
        Self {
            configured_target: configured_target.to_owned(),
            category: category.to_owned(),
            identifier: identifier.to_owned(),
            action,
        }
    }

    /// Create a LogicalActionId from a Buck2 event
    pub fn from_event(event: &Event) -> Option<Self> {
        let target_info = event.extract_target_info()?;

        let target_id = TargetBuildId::new(&target_info.package, &target_info.name);
        let configured_id = ConfiguredTargetId::new(&target_id, &target_info.configuration);

        Some(Self::new(
            &configured_id,
            &target_info.action_name.category,
            &target_info.action_name.identifier,
            target_info.action_kind,
        ))
    }
}

/// Buck2 build events.
///
/// Buck2 emits a stream of build events that can be consumed for observability,
/// logging, and analysis purposes. This enum represents the various types of
/// events that can occur during a build.
///
/// For more details about Buck2 events and their structure, see:
/// - Buck2 documentation: https://buck2.build/docs/users/build_observability/logging/#buck-events
/// - Protocol Buffers definitions: https://github.com/facebook/buck2/blob/main/app/buck2_data/data.proto
///
/// # Example Events
/// - `ActionExecutionStart`: Emitted when an action begins execution
/// - `ActionExecutionEnd`: Emitted when an action completes execution
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct Event {
    #[serde(rename = "Event")]
    pub event: BuckEvent,
}

// Target info in buck2 build
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub package: String,
    pub name: String,
    pub configuration: String,
    pub action_kind: ActionKind,
    pub action_name: ActionName,
}

impl Event {
    /// Extract target information from the event
    pub fn extract_target_info(&self) -> Option<TargetInfo> {
        match &self.event.data {
            SpanWrapper::SpanStart { span_start } => {
                let ae = &span_start.data.action_execution;
                Self::extract_from_action_execution(ae)
            }
            SpanWrapper::SpanEnd { span_end } => {
                let ae = &span_end.data.action_execution;
                Self::extract_from_action_execution(ae)
            }
        }
    }

    /// Extract from ActionExecutionContent (internal helper)
    fn extract_from_action_execution(ae: &ActionExecutionContent) -> Option<TargetInfo> {
        let (package, name, configuration) = match &ae.action_key.action_owner {
            OwnerWrapper::TargetLabel { target_label } => (
                target_label.label.package.clone(),
                target_label.label.name.clone(),
                target_label.configuration.full_name.clone(),
            ),
            OwnerWrapper::Direct(target_label) => (
                target_label.label.package.clone(),
                target_label.label.name.clone(),
                target_label.configuration.full_name.clone(),
            ),
        };

        Some(TargetInfo {
            package,
            name,
            configuration,
            action_kind: ae.action_kind.clone(),
            action_name: ae.action_name.clone(),
        })
    }

    /// Get the execution status based on event type and action execution data
    pub fn execution_status(&self) -> Option<ExecutionStatus> {
        match &self.event.data {
            SpanWrapper::SpanStart { .. } => Some(ExecutionStatus::Running),
            SpanWrapper::SpanEnd { span_end } => {
                let is_fail = span_end.data.action_execution.is_fail.unwrap_or(false);

                Some(if is_fail {
                    ExecutionStatus::Failed
                } else {
                    ExecutionStatus::Succeeded
                })
            }
        }
    }

    /// Get the action execution content if this is an action event
    pub fn action_execution(&self) -> Option<&ActionExecutionContent> {
        match &self.event.data {
            SpanWrapper::SpanStart { span_start } => Some(&span_start.data.action_execution),
            SpanWrapper::SpanEnd { span_end } => Some(&span_end.data.action_execution),
        }
    }

    /// Extract only package and name
    pub fn extract_target_id(&self) -> Option<(String, String)> {
        self.extract_target_info()
            .map(|info| (info.package, info.name))
    }

    /// Extract only configuration
    pub fn extract_configuration(&self) -> Option<String> {
        self.extract_target_info().map(|info| info.configuration)
    }

    /// Check if this is a SpanStart event
    pub fn is_start(&self) -> bool {
        matches!(self.event.data, SpanWrapper::SpanStart { .. })
    }

    /// Check if this is a SpanEnd event
    pub fn is_end(&self) -> bool {
        matches!(self.event.data, SpanWrapper::SpanEnd { .. })
    }

    /// Check if this is a failed action
    pub fn is_failed(&self) -> Option<bool> {
        match &self.event.data {
            SpanWrapper::SpanEnd { span_end } => span_end.data.action_execution.is_fail,
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuckEvent {
    pub timestamp: Option<(i64, i64)>,
    /// Distributed tracing identifier
    pub trace_id: String,
    /// Current span identifier
    pub span_id: usize,
    /// Parent span identifier
    pub parent_id: usize,
    /// Span data (start or end event)
    pub data: SpanWrapper,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SpanWrapper {
    SpanStart {
        #[serde(rename = "SpanStart")]
        span_start: SpanStart,
    },
    SpanEnd {
        #[serde(rename = "SpanEnd")]
        span_end: SpanEnd,
    },
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct SpanStart {
    pub data: ActionExecutionData,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct SpanEnd {
    pub stats: Option<HashMap<String, u64>>,
    pub duration_us: u64,
    pub data: ActionExecutionData,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ActionExecutionData {
    #[serde(rename = "ActionExecution")]
    pub action_execution: ActionExecutionContent,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ActionExecutionContent {
    /// A unique key identifying this action within the build.
    #[serde(rename = "key")]
    pub action_key: ActionKey,

    /// The type of action being executed (e.g., COPY, RUN, WRITE).
    /// Determines how the action is processed and what resources it requires.
    #[serde(rename = "kind")]
    pub action_kind: ActionKind,

    /// This typically describes what the action does (e.g., "C++ compile", "Java test").
    #[serde(rename = "name")]
    pub action_name: ActionName,

    /// Indicates whether the action execution failed.
    /// - `true`: Action completed with an error
    /// - `false`: Action completed successfully
    #[serde(rename = "failed")]
    pub is_fail: Option<bool>,

    /// Error message if the action failed.
    #[serde(rename = "error")]
    pub error_message: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Unique identifier for an action within a Buck2 build.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ActionKey {
    /// Internal action identifier.
    pub id: String,

    /// Human-readable action key string.
    pub key: String,

    /// Configured target that owns this action.
    #[serde(rename = "owner")]
    pub action_owner: OwnerWrapper,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum OwnerWrapper {
    /// Target label wrapper with configuration
    TargetLabel {
        #[serde(rename = "TargetLabel")]
        target_label: TargetBuildLabel,
    },
    /// Direct target label (for other JSON formats)
    Direct(TargetBuildLabel),
}

/// Buck2 target identifier with configuration.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct TargetBuildLabel {
    pub label: Label,
    pub configuration: Configuration,

    /// Optional execution configuration
    #[serde(rename = "execution_configuration")]
    pub execution_configuration: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Target label (package + name).
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct Label {
    /// Package path (e.g., "root//third-party/rust/crates/flate2/1.1.2")
    pub package: String,
    /// Target name (e.g., "flate2")
    pub name: String,
}

/// Build configuration identifier.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct Configuration {
    /// Full configuration name (e.g., "prelude//platforms:default#b42aeba648b8c415")
    #[serde(rename = "full_name")]
    pub full_name: String,
}

/// Type of action being executed.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum ActionKind {
    NotSet,
    Copy,
    DownloadFile,
    Run,
    SymlinkedDir,
    Write,
    WriteMacrosToFile,
    CasArtifact,
    /// Catch-all for other action kinds
    #[serde(other)]
    Other,
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ActionKind::NotSet => "not_set",
            ActionKind::Copy => "copy",
            ActionKind::DownloadFile => "download_file",
            ActionKind::Run => "run",
            ActionKind::SymlinkedDir => "symlinked_dir",
            ActionKind::Write => "write",
            ActionKind::WriteMacrosToFile => "write_macros_to_file",
            ActionKind::CasArtifact => "cas_artifact",
            ActionKind::Other => "other",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ActionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "not_set" | "notset" | "not-set" => Ok(ActionKind::NotSet),
            "copy" => Ok(ActionKind::Copy),
            "download_file" | "downloadfile" | "download-file" => Ok(ActionKind::DownloadFile),
            "run" => Ok(ActionKind::Run),
            "symlinked_dir" | "symlinkeddir" | "symlinked-dir" => Ok(ActionKind::SymlinkedDir),
            "write" => Ok(ActionKind::Write),
            "write_macros_to_file" | "writemacrostofile" | "write-macros-to-file" => {
                Ok(ActionKind::WriteMacrosToFile)
            }
            "cas_artifact" | "casartifact" | "cas-artifact" => Ok(ActionKind::CasArtifact),
            "other" => Ok(ActionKind::Other),
            _ => Err(format!("Unknown action kind: '{}'", s)),
        }
    }
}

/// Human-readable action name for display and analytics.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ActionName {
    /// Action category (e.g., "cxx_compile", "write_json")
    pub category: String,
    /// Unique identifier within the category
    pub identifier: String,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

    use super::*;

    #[test]
    fn test_parse_buck_events() {
        // Raw Buck2 Event (SpanEnd/ActionExecution)
        let json_end = r#"
        {
            "Event": {
                "timestamp": [
                    1770640053,
                    612112365
                ],
                "trace_id": "885c7166-ea44-4eaf-b9bc-065ee405ebb1",
                "span_id": 20882,
                "parent_id": 0,
                "data": {
                    "SpanEnd": {
                        "stats": {
                            "max_poll_time_us": 28155,
                            "total_poll_time_us": 28158
                        },
                        "duration_us": 954356,
                        "data": {
                            "ActionExecution": {
                                "key": {
                                    "id": "0600000000000000",
                                    "key": "_6",
                                    "owner": {
                                        "TargetLabel": {
                                            "label": {
                                                "package": "root//third-party/rust/crates/generic-array/0.14.7",
                                                "name": "generic-array-build-script-build"
                                            },
                                            "configuration": {
                                                "full_name": "prelude//platforms:default#f44b4d88cb2902e8"
                                            },
                                            "execution_configuration": null
                                        }
                                    }
                                },
                                "kind": "Write",
                                "name": {
                                    "category": "write",
                                    "identifier": "XIPL/build_script_build-link-diag.args"
                                },
                                "failed": false,
                                "always_print_stderr": false,
                                "execution_kind": 4,
                                "wall_time_us": 28053,
                                "output_size": 2431,
                                "commands": [],
                                "outputs": [
                                    {
                                        "tiny_digest": "2b685d67"
                                    }
                                ],
                                "prefers_local": false,
                                "requires_local": false,
                                "allows_cache_upload": false,
                                "did_cache_upload": false,
                                "buck2_revision": null,
                                "buck2_build_time": null,
                                "eligible_for_full_hybrid": null,
                                "hostname": null,
                                "allows_dep_file_cache_upload": false,
                                "did_dep_file_cache_upload": false,
                                "dep_file_key": null,
                                "error_diagnostics": null,
                                "input_files_bytes": null,
                                "invalidation_info": null,
                                "target_rule_type_name": "rust_binary",
                                "scheduling_mode": null,
                                "incremental_kind": null,
                                "eligible_for_dedupe": 2,
                                "expected_eligible_for_dedupe": 2,
                                "error": null
                            }
                        }
                    }
                }
            }
        }
        "#;

        // Try to parse SpanEnd
        let parsed_end: Result<Event, serde_json::Error> = from_str(json_end);

        // Print error information if any
        match &parsed_end {
            Ok(event) => {
                println!("✅ SpanEnd JSON parsed successfully!");
                println!("Trace ID: {}", event.event.trace_id);
                println!("Span ID: {}", event.event.span_id);

                if let SpanWrapper::SpanEnd { span_end } = &event.event.data {
                    println!("Duration: {} us", span_end.duration_us);

                    // Verify action execution data
                    let action_exec = &span_end.data.action_execution;
                    println!("Action kind: {:?}", action_exec.action_kind);
                    println!("Action category: {}", action_exec.action_name.category);
                    println!("Failed: {:?}", action_exec.is_fail);

                    // Verify owner parsing
                    if let OwnerWrapper::TargetLabel { target_label } =
                        &action_exec.action_key.action_owner
                    {
                        println!("Owner package: {}", target_label.label.package);
                        println!("Owner name: {}", target_label.label.name);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to parse SpanEnd JSON: {}", e);
                println!("Error location: line {}, column {}", e.line(), e.column());
                println!("Full error: {:#?}", e);
            }
        }

        assert!(parsed_end.is_ok(), "SpanEnd JSON should parse successfully");

        // Raw Buck2 Event (SpanStart/ActionExecution)
        let json_start = r#"
        {
            "Event": {
                "timestamp": [
                    1770640053,
                    579589525
                ],
                "trace_id": "885c7166-ea44-4eaf-b9bc-065ee405ebb1",
                "span_id": 37915,
                "parent_id": 0,
                "data": {
                    "SpanStart": {
                        "data": {
                            "ActionExecution": {
                                "key": {
                                    "id": "0300000000000000",
                                    "key": "_3",
                                    "owner": {
                                        "TargetLabel": {
                                            "label": {
                                                "package": "root//third-party/rust/crates/flate2/1.1.2",
                                                "name": "flate2"
                                            },
                                            "configuration": {
                                                "full_name": "prelude//platforms:default#b42aeba648b8c415"
                                            }
                                        }
                                    }
                                },
                                "kind": "Write",
                                "name": {
                                    "category": "write_json",
                                    "identifier": "LPPM-depsfast-symlinked_dirs.json"
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        // Try to parse SpanStart
        let parsed_start: Result<Event, serde_json::Error> = from_str(json_start);

        match &parsed_start {
            Ok(event) => {
                println!("✅ SpanStart JSON parsed successfully!");
                println!("Trace ID: {}", event.event.trace_id);

                if let SpanWrapper::SpanStart { span_start } = &event.event.data {
                    let action_exec = &span_start.data.action_execution;
                    println!("Action kind: {:?}", action_exec.action_kind);
                    println!("Action category: {}", action_exec.action_name.category);
                    println!("Action identifier: {}", action_exec.action_name.identifier);
                }
            }
            Err(e) => {
                println!("❌ Failed to parse SpanStart JSON: {}", e);
                println!("Error location: line {}, column {}", e.line(), e.column());
            }
        }

        assert!(
            parsed_start.is_ok(),
            "SpanStart JSON should parse successfully"
        );

        // Verify the parsed event data
        let event_end = parsed_end.unwrap().event;
        assert_eq!(event_end.trace_id, "885c7166-ea44-4eaf-b9bc-065ee405ebb1");
        assert_eq!(event_end.span_id, 20882);

        // Verify the structure of SpanEnd
        match event_end.data {
            SpanWrapper::SpanEnd { span_end } => {
                assert_eq!(span_end.duration_us, 954356);
                assert_eq!(span_end.data.action_execution.action_name.category, "write");
                assert_eq!(span_end.data.action_execution.is_fail, Some(false));
            }
            _ => panic!("Expected SpanEnd, got something else"),
        }

        let event_start = parsed_start.unwrap().event;
        assert_eq!(event_start.trace_id, "885c7166-ea44-4eaf-b9bc-065ee405ebb1");

        // Verify the structure of SpanStart
        match event_start.data {
            SpanWrapper::SpanStart { span_start } => {
                assert_eq!(
                    span_start.data.action_execution.action_name.category,
                    "write_json"
                );
                assert_eq!(
                    span_start.data.action_execution.action_name.identifier,
                    "LPPM-depsfast-symlinked_dirs.json"
                );
            }
            _ => panic!("Expected SpanStart, got something else"),
        }
    }

    #[test]
    fn test_parse_minimal_span_end() {
        // Test with minimal required fields
        let minimal_json = r#"
        {
            "Event": {
                "trace_id": "test-trace-id",
                "span_id": 12345,
                "parent_id": 0,
                "data": {
                    "SpanEnd": {
                        "duration_us": 1000,
                        "data": {
                            "ActionExecution": {
                                "key": {
                                    "id": "test-id",
                                    "key": "test-key",
                                    "owner": {
                                        "TargetLabel": {
                                            "label": {
                                                "package": "test//package",
                                                "name": "test-target"
                                            },
                                            "configuration": {
                                                "full_name": "test//config"
                                            }
                                        }
                                    }
                                },
                                "kind": "Write",
                                "name": {
                                    "category": "test",
                                    "identifier": "test-id"
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let parsed: Result<Event, _> = from_str(minimal_json);
        assert!(
            parsed.is_ok(),
            "Minimal SpanEnd JSON should parse successfully"
        );
    }

    #[test]
    fn test_action_kind_serialization() {
        // Test that ActionKind serializes/deserializes correctly with PascalCase
        let json = r#""Write""#;
        let kind: ActionKind = from_str(json).unwrap();
        assert_eq!(kind, ActionKind::Write);

        let serialized = serde_json::to_string(&ActionKind::Copy).unwrap();
        assert_eq!(serialized, r#""Copy""#);
    }
}
