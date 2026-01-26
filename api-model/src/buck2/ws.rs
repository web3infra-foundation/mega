//! Websocket message protocol for communication between orion worker and server.

use crate::buck2::types::{ProjectRelativePath, Status, TaskPhase};
use serde::{Deserialize, Serialize};

/// Message protocol for WebSocket communication between worker and server.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WSMessage {
    // Server -> Worker messages
    Task {
        id: String,
        repo: String,
        cl_link: String,
        changes: Vec<Status<ProjectRelativePath>>,
    },

    TaskWithTargets {
        id: String,
        repo: String,
        cl_link: String,
        // targets: Vec<Target>,
    },

    // Worker -> Server messages
    Register {
        id: String,
        hostname: String,
        orion_version: String,
    },

    Heartbeat,
    // Sent when a task is in the build process and its execution phase changes.
    TaskPhaseUpdate {
        id: String,
        phase: TaskPhase,
    },

    TaskAck {
        id: String,
        success: bool,
        message: String,
    },

    TaskBuildOutput {
        id: String,
        output: String,
    },

    TaskBuildComplete {
        id: String,
        success: bool,
        exit_code: Option<i32>,
        message: String,
    },
}
