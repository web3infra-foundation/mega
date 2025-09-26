use common::config::BlameConfig;
use mercury::hash::SHA1;
use serde::{Deserialize, Serialize};

/// Configuration for handling large files and performance tuning in blame operations.
///
/// These settings act as safeguards to prevent the service from consuming excessive
/// resources when a user requests a blame on a very large or complex file. They are
/// typically loaded from a central application configuration file.
#[derive(Debug, Clone)]
pub struct LargeFileConfig {
    /// The maximum number of lines a file can have before it is considered "large".
    /// Blame operations on files exceeding this limit might be rejected or handled
    /// by a special streaming process to conserve memory.
    /// Default: 1000
    pub max_lines_threshold: usize,

    /// The maximum file size in bytes. Files larger than this are considered "large".
    /// This is a primary guard against memory exhaustion from loading very large files.
    /// Default: 1,048,576 (1MB)
    pub max_size_threshold: usize,

    /// The default number of lines to process in a single chunk when using
    /// streaming blame for large files. Smaller chunks use less memory but may
    /// increase processing time.
    /// Default: 100
    pub default_chunk_size: usize,

    /// The maximum number of commit objects to hold in memory at one time during
    /// the history traversal. This helps to limit memory usage when blaming files
    /// with very deep or complex histories.
    /// Default: 50
    pub max_commits_in_memory: usize,

    /// A master switch to enable or disable the caching of intermediate blame results,
    /// such as commit data and file versions. Disabling this can be useful for debugging
    /// but will significantly degrade performance on repeated requests.
    /// Default: true
    pub enable_caching: bool,
}

impl Default for LargeFileConfig {
    fn default() -> Self {
        Self {
            max_lines_threshold: 1000,
            max_size_threshold: 1024 * 1024, // 1MB
            default_chunk_size: 100,
            max_commits_in_memory: 50,
            enable_caching: true,
        }
    }
}

impl From<&BlameConfig> for LargeFileConfig {
    /// Creates a LargeFileConfig from the global BlameConfig
    fn from(config: &BlameConfig) -> Self {
        Self {
            max_lines_threshold: config.max_lines_threshold,
            max_size_threshold: config.get_max_size_bytes().unwrap_or(1024 * 1024),
            default_chunk_size: config.default_chunk_size,
            max_commits_in_memory: config.max_commits_in_memory,
            enable_caching: config.enable_caching,
        }
    }
}

/// Line attribution information for blame tracking
#[derive(Debug, Clone)]
pub struct LineAttribution {
    pub line_number: usize,
    pub content: String,
    pub commit_hash: SHA1,
    pub line_number_in_commit: usize,
}

/// File version for blame analysis
#[derive(Debug, Clone)]
pub struct FileVersion {
    pub commit_hash: SHA1,
    pub blob_hash: SHA1,
    pub content: String,
    pub lines: Vec<String>,
}

/// Blame information for a specific commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameInfo {
    pub commit_hash: String,
    pub commit_short_id: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_email: String,
    pub committer_time: i64,
    pub commit_message: String,
    pub commit_summary: String,
    pub original_line_number: usize,
    // Campsite username fields for frontend to query user info via other APIs
    pub author_username: Option<String>,
    pub committer_username: Option<String>,
    pub commit_detail_url: String,
}

/// Individual line blame information (used internally for processing)
#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line_number: usize,
    pub content: String,
    pub blame_info: BlameInfo,
}

/// Represents a continuous block of lines attributed to the same commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameBlock {
    pub content: String,
    pub blame_info: BlameInfo,
    pub start_line: usize,
    pub end_line: usize,
    pub line_count: usize,
}

/// Complete blame result for a file
#[derive(Debug, Serialize, Deserialize)]
pub struct BlameResult {
    pub file_path: String,
    pub blocks: Vec<BlameBlock>,
    pub total_lines: usize,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    /// Earliest commit time across all lines in the file (Unix timestamp)
    pub earliest_commit_time: i64,
    /// Latest commit time across all lines in the file (Unix timestamp)
    pub latest_commit_time: i64,
    /// List of contributors to this file
    pub contributors: Vec<Contributor>,
}

/// Query parameters for blame requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameQuery {
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

/// A structure for tracking blame candidates during history traversal
#[derive(Debug, Clone)]
pub struct BlameCandidate {
    /// Line number in the current commit (1-based)
    pub line_number: usize,
    /// Original line number in the final file (1-based)
    pub original_final_line_number: usize,
}

/// Contributor information including campsite username
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contributor {
    pub email: String,
    pub username: Option<String>,
    pub last_commit_time: i64,
    pub total_lines: usize,
}
