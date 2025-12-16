/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::process::Termination;

use thiserror::Error;

// Exit codes are handled by the [orchestrator classifiers](https://fburl.com/code/k33ivq2j)
const EXIT_CODE_WARNING: u8 = 2;
const EXIT_CODE_SKIPPED: u8 = 3;
const EXIT_CODE_USER_FAILURE: u8 = 4;
const EXIT_CODE_INFRA_FAILURE: u8 = 5;

// Error type for workflow errors.
// Supports setting a workflow status by using custom constructors for [`Self::ReturnStatus`]
// For compatibility with `?` operator, [`Self::Other`] is used as a fallback.
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("{1}")]
    ReturnStatus(String, u8),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl WorkflowError {
    pub fn warning(message: String) -> Self {
        Self::ReturnStatus(message, EXIT_CODE_WARNING)
    }

    pub fn skipped(message: String) -> Self {
        Self::ReturnStatus(message, EXIT_CODE_SKIPPED)
    }

    pub fn user_failure(message: String) -> Self {
        Self::ReturnStatus(message, EXIT_CODE_USER_FAILURE)
    }

    pub fn infra_failure(message: String) -> Self {
        Self::ReturnStatus(message, EXIT_CODE_INFRA_FAILURE)
    }
}

impl Termination for WorkflowError {
    fn report(self) -> std::process::ExitCode {
        match self {
            Self::ReturnStatus(message, code) => {
                eprintln!("\n----------------------------------------");
                eprintln!("{}", message);
                std::process::ExitCode::from(code)
            }
            Self::Other(err) => {
                eprintln!("Error executing: {:#}", err);
                std::process::ExitCode::FAILURE
            }
        }
    }
}
