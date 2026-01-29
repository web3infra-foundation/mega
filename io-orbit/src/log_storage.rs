//! Log storage abstraction over [`MegaObjectStorage`].
//!
//! Each log is identified by an [`ObjectKey`] (e.g. `{task_id}/{repo_name}/{build_id}`).
//! Data is stored as segments; a manifest per log holds `len` and segment metadata.

use common::errors::MegaError;
use serde::{Deserialize, Serialize};

use crate::object_storage::{MegaObjectStorage, ObjectByteStream, ObjectKey, ObjectMeta};

/// Manifest for a single log stream.
///
/// Serialized (e.g. JSON) and stored as one object per log. Tracks current length
/// and the list of segments that make up the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogManifest {
    /// Current log length in bytes (= next append offset). Incremented on each append.
    pub len: u64,

    /// Ordered list of segments. Each segment covers `[offset, offset + len)`.
    /// TODO: Future extensions to manifest (e.g., checksum, compression flags, etc.).
    pub segments: Vec<LogSegmentMeta>,
}

/// Metadata for one segment of a log.
///
/// Segment data is stored as a separate object; this struct records its position
/// in the logical log and its storage key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSegmentMeta {
    /// Start offset of this segment in the log (bytes).
    pub offset: u64,

    /// Length of this segment in bytes.
    pub len: u64,

    /// Object storage key for this segment (e.g. `{log_key}/segments/{offset}-{end}` or custom).
    pub key: String,
}

/// Log storage trait: append-only logs over [`MegaObjectStorage`].
///
/// All methods take `key: &ObjectKey` as the **log identifier** (e.g.
/// `{task_id}/{repo_name}/{build_id}`). It identifies the log as a whole, **not**
/// a segment. Segment keys live in [`LogSegmentMeta::key`].
#[async_trait::async_trait]
pub trait LogStorage: MegaObjectStorage {
    /// Appends `data` to the end of the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - Log identifier (e.g. `task_id/repo_name/build_id`). Not a segment key.
    /// * `data` - Byte stream to append.
    /// * `meta` - Optional metadata for the append.
    ///
    /// # Returns
    /// * `Ok(len)` - New log length in bytes after append.
    async fn append(
        &self,
        key: &ObjectKey,
        data: ObjectByteStream,
        meta: ObjectMeta,
    ) -> Result<(), MegaError>;

    /// Reads the byte range `[offset, offset + length)` from the log identified by `key`.
    ///
    /// # Arguments
    /// * `key` - Log identifier. Not a segment key.
    /// * `offset` - Start byte offset (inclusive).
    /// * `length` - Number of bytes to read.
    ///
    /// # Returns
    /// * `Ok(stream)` - Byte stream for the requested range.
    async fn read_range(
        &self,
        key: &ObjectKey,
        offset: u64,
        length: u64,
    ) -> Result<ObjectByteStream, MegaError>;

    async fn read_lines_range(
        &self,
        key: &ObjectKey,
        start_line: u64,
        end_line: u64,
    ) -> Result<ObjectByteStream, MegaError>;

    /// Appends `data` only if the current log length equals `expected_offset`.
    ///
    /// Used for linearizable append (e.g. single writer or CAS). Returns an error
    /// if the actual length does not match.
    ///
    /// # Arguments
    /// * `key` - Log identifier. Not a segment key.
    /// * `expected_offset` - Current length must equal this, else `Err`.
    /// * `data` - Byte stream to append.
    /// * `meta` - Optional metadata.
    ///
    /// # Returns
    /// * `Ok(len)` - New log length after append.
    async fn append_concurrently(
        &self,
        key: &ObjectKey,
        //expected_offset: u64,
        data: ObjectByteStream,
        meta: ObjectMeta,
    ) -> Result<(), MegaError>;

    /// Loads the manifest for the log identified by `key` and returns its length.
    ///
    /// # Arguments
    /// * `key` - Log identifier. Not a segment key.
    ///
    /// # Returns
    /// * `Ok(len)` - Current log length (`LogManifest::len`). `0` if manifest does not exist.
    async fn load_manifest(&self, key: &ObjectKey) -> Result<LogManifest, MegaError>;

    /// Checks whether the log identified by `key` exists.
    ///
    /// Implementations are free to decide what "exists" means (e.g. manifest present,
    /// segments present, etc.). Callers should not rely on manifest/segment layout.
    async fn log_exists(&self, key: &ObjectKey) -> Result<bool, MegaError>;
}
