//! yaml configuration file type parser
//!
//! # Config file parser
//!
//! Use yaml configuration files to define a series of tasks, which eliminates the need for users to write code.
//! [`YamlParser`] is responsible for parsing the yaml configuration file into a series of [`YamlTask`].
//! The program specifies the properties of the yaml task configuration file. The basic format of the yaml
//! configuration file is as follows:
//!
//! ```yaml
//! dagrs:
//!   a:
//!     name: "Task 1"
//!     after: [ b, c ]
//!     cmd: echo a
//!   b:
//!     name: "Task 2"
//!     after: [ c, f, g ]
//!     cmd: echo b
//!   c:
//!     name: "Task 3"
//!     after: [ e, g ]
//!     cmd: echo c
//!   d:
//!     name: "Task 4"
//!     after: [ c, e ]
//!     cmd: echo d
//!   e:
//!     name: "Task 5"
//!     after: [ h ]
//!     cmd: echo e
//!   f:
//!     name: "Task 6"
//!     after: [ g ]
//!     cmd: python3 ./tests/config/test.py
//!   g:
//!     name: "Task 7"
//!     after: [ h ]
//!     cmd: node ./tests/config/test.js
//!   h:
//!     name: "Task 8"
//!     cmd: echo h
//! ```
//!
//! Users can read the yaml configuration file programmatically or by using the compiled `dagrs`
//! command line tool. Either way, you need to enable the `yaml` feature.
//!
//! # Example
//!
//! ```rust
//! use dagrs::Dag;
//! let dag = Dag::with_yaml("some_path",std::collections::HashMap::new());
//! ```

mod yaml_parser;
mod yaml_task;

use crate::DagError;
use thiserror::Error;

pub use self::yaml_parser::YamlParser;
pub use self::yaml_task::YamlTask;

/// Errors about task configuration items.
#[derive(Debug, Error)]
pub enum YamlTaskError {
    /// The configuration file should start with `dagrs:`.
    #[error("File content is not start with 'dagrs'.")]
    StartWordError,
    /// No task name configured.
    #[error("Task has no name field. [{0}]")]
    NoNameAttr(String),
    /// The specified task predecessor was not found.
    #[error("Task cannot find the specified predecessor. [{0}]")]
    NotFoundPrecursor(String),
    /// `script` is not defined.
    #[error("The 'script' attribute is not defined. [{0}]")]
    NoScriptAttr(String),
}

/// Error about file information.
#[derive(Debug, Error)]
pub enum FileContentError {
    /// The format of the yaml configuration file is not standardized.
    #[error("Illegal yaml content: {0}")]
    IllegalYamlContent(yaml_rust::ScanError),
    /// The file is empty.
    #[error("File is empty! [{0}]")]
    Empty(String),
}

/// Configuration file not found.
#[derive(Debug, Error)]
#[error("File not found. [{0}]")]
pub struct FileNotFound(pub std::io::Error);

impl From<YamlTaskError> for DagError {
    fn from(value: YamlTaskError) -> Self {
        DagError::ParserError(value.to_string())
    }
}

impl From<FileContentError> for DagError {
    fn from(value: FileContentError) -> Self {
        DagError::ParserError(value.to_string())
    }
}

impl From<FileNotFound> for DagError {
    fn from(value: FileNotFound) -> Self {
        DagError::ParserError(value.to_string().into())
    }
}
