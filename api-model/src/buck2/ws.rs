//! Websocket message protocol for communication between orion worker and server.

use serde::{Deserialize, Serialize};

use crate::buck2::{
    status::Status,
    types::{ProjectRelativePath, TaskPhase},
};

/// Message protocol for WebSocket communication between worker and server.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WSMessage {
    // Server -> Worker messages
    TaskBuild {
        build_id: String,
        repo: String,
        cl_link: String,
        changes: Vec<Status<ProjectRelativePath>>,
    },

    TaskBuildWithTargets {
        build_id: String,
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

    // Sent when a task is in the build process and its execution phase changes.
    Heartbeat,

    TaskPhaseUpdate {
        build_id: String,
        phase: TaskPhase,
    },

    TaskAck {
        build_id: String,
        success: bool,
        message: String,
    },

    TaskBuildOutput {
        build_id: String,
        output: String,
    },

    TaskBuildComplete {
        build_id: String,
        success: bool,
        exit_code: Option<i32>,
        message: String,
    },
}
