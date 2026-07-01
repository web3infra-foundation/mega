//! Diff and patch operations for [`ClApplicationService`](super::service::ClApplicationService).

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use api_model::common::Pagination;
use common::errors::MegaError;
use git_internal::{
    DiffItem, diff::Diff as GitDiff, errors::GitError, hash::ObjectHash,
    internal::object::tree::Tree,
};
use jupiter::utils::converter::FromMegaModel;

use crate::{
    application::api_service::{ApiHandler, mono::ClApplicationService},
    diff::tree_diff,
    model::change_list::{ClDiffFile, ClFilesChangedItemSchema},
};

const LARGE_CL_RENAME_DETECTION_THRESHOLD: usize = 1000;

struct PatchSections<'a> {
    header_lines: Vec<&'a str>,
    divider_line: Option<&'a str>,
    payload_lines: Vec<&'a str>,
    has_trailing_newline: bool,
}

struct PagedClDiffItem {
    item: DiffItem,
    old_path: Option<String>,
}

impl ClApplicationService {
    /// Fetches the content difference for a merge request, paginated by page_id and page_size.
    /// # Arguments
    /// * `cl_link` - The link to the merge request.
    /// * `page_id` - The page number to fetch. (id out of bounds will return empty)
    /// * `page_size` - The number of items per page.
    /// # Returns
    ///  a `Result` containing `ClDiff` on success or a `GitError` on failure.
    /// Build paged CL diff items with optional relocation metadata for CL views.
    async fn paged_content_diff_items(
        &self,
        cl_link: &str,
        page: Pagination,
    ) -> Result<(Vec<PagedClDiffItem>, u64), GitError> {
        let per_page = page.per_page as usize;
        let page_id = page.page as usize;

        let stg = self.storage().cl_service.cl_store();
        let cl =
            stg.get_cl(cl_link).await.unwrap().ok_or_else(|| {
                GitError::CustomError(format!("Merge request not found: {cl_link}"))
            })?;
        let old_blobs = self
            .get_commit_blobs(&cl.from_hash)
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get old commit blobs: {e}")))?;
        let new_blobs = self
            .get_commit_blobs(&cl.to_hash)
            .await
            .map_err(|e| GitError::CustomError(format!("Failed to get new commit blobs: {e}")))?;

        let sorted_changed_files = self
            .cl_files_list(old_blobs, new_blobs)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?;

        let start = (page_id.saturating_sub(1)) * per_page;
        let end = (start + per_page).min(sorted_changed_files.len());

        let page_slice: &[ClDiffFile] = if start < sorted_changed_files.len() {
            let start_idx = start;
            let end_idx = end;
            &sorted_changed_files[start_idx..end_idx]
        } else {
            &[]
        };

        let non_relocated_items: Vec<ClDiffFile> = page_slice
            .iter()
            .filter(|item| {
                !matches!(
                    item,
                    ClDiffFile::Renamed(_, _, _, _, _) | ClDiffFile::Moved(_, _, _, _, _)
                )
            })
            .cloned()
            .collect();

        let mut page_old_blobs = Vec::new();
        let mut page_new_blobs = Vec::new();
        collect_page_blobs(
            &non_relocated_items,
            &mut page_old_blobs,
            &mut page_new_blobs,
        );

        let raw_diff_output = if non_relocated_items.is_empty() {
            Vec::new()
        } else {
            self.get_diff_by_blobs(page_old_blobs, page_new_blobs)
                .await?
        };

        let mut raw_diff_by_path: HashMap<String, Vec<DiffItem>> = HashMap::new();
        for item in raw_diff_output {
            raw_diff_by_path
                .entry(item.path.clone())
                .or_default()
                .push(item);
        }

        let mut diff_output: Vec<PagedClDiffItem> = Vec::with_capacity(page_slice.len());
        for item in page_slice {
            match item {
                ClDiffFile::Renamed(old_path, new_path, old_hash, new_hash, similarity)
                | ClDiffFile::Moved(old_path, new_path, old_hash, new_hash, similarity) => {
                    diff_output.push(PagedClDiffItem {
                        item: self
                            .format_relocated_diff_item(
                                old_path,
                                new_path,
                                *old_hash,
                                *new_hash,
                                *similarity,
                            )
                            .await?,
                        old_path: Some(old_path.to_string_lossy().replace('\\', "/")),
                    });
                }
                _ => {
                    let key = item.path().to_string_lossy().replace('\\', "/");
                    if let Some(items) = raw_diff_by_path.get_mut(&key)
                        && !items.is_empty()
                    {
                        diff_output.push(PagedClDiffItem {
                            item: items.remove(0),
                            old_path: None,
                        });
                    }
                }
            }
        }

        let total = sorted_changed_files.len().div_ceil(per_page);

        Ok((diff_output, total as u64))
    }

    /// Return the legacy paged diff shape without CL-specific metadata.
    pub async fn paged_content_diff(
        &self,
        cl_link: &str,
        page: Pagination,
    ) -> Result<(Vec<DiffItem>, u64), GitError> {
        let (items, total) = self.paged_content_diff_items(cl_link, page).await?;
        Ok((items.into_iter().map(|item| item.item).collect(), total))
    }

    /// Return paged diff items tailored for the CL files-changed API.
    pub async fn paged_content_diff_for_cl(
        &self,
        cl_link: &str,
        page: Pagination,
    ) -> Result<(Vec<ClFilesChangedItemSchema>, u64), GitError> {
        let (items, total) = self.paged_content_diff_items(cl_link, page).await?;
        Ok((
            items
                .into_iter()
                .map(|item| ClFilesChangedItemSchema::new(item.item, item.old_path))
                .collect(),
            total,
        ))
    }

    async fn get_diff_by_blobs(
        &self,
        old_blobs: Vec<(PathBuf, ObjectHash)>,
        new_blobs: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<DiffItem>, GitError> {
        let mut blob_cache: HashMap<ObjectHash, Vec<u8>> = HashMap::new();

        // Collect all unique hashes
        let mut all_hashes = HashSet::new();
        for (_, hash) in &old_blobs {
            all_hashes.insert(*hash);
        }
        for (_, hash) in &new_blobs {
            all_hashes.insert(*hash);
        }

        // Fetch all blobs with better error handling and logging
        let mut failed_hashes = Vec::new();
        for hash in all_hashes {
            match self.git().get_raw_blob_by_hash(&hash.to_string()).await {
                Ok(data) => {
                    blob_cache.insert(hash, data);
                }
                Err(e) => {
                    tracing::error!("Failed to fetch blob {}: {}", hash, e);
                    failed_hashes.push(hash);
                    blob_cache.insert(hash, Vec::new());
                }
            }
        }

        if !failed_hashes.is_empty() {
            tracing::warn!(
                "Failed to fetch {} blob(s): {:?}",
                failed_hashes.len(),
                failed_hashes
            );
        }

        // Enhanced content reader with better error handling
        let read_content = |file: &PathBuf, hash: &ObjectHash| -> Vec<u8> {
            match blob_cache.get(hash) {
                Some(content) => content.clone(),
                None => {
                    tracing::warn!("Missing blob content for file: {:?}, hash: {}", file, hash);
                    Vec::new()
                }
            }
        };

        // Use the unified diff function with configurable algorithm
        let diff_output = GitDiff::diff(old_blobs, new_blobs, Vec::new(), read_content)
            .into_iter()
            .map(Self::normalize_diff_item)
            .collect();

        Ok(diff_output)
    }

    async fn format_relocated_diff_item(
        &self,
        old_path: &Path,
        new_path: &Path,
        old_hash: ObjectHash,
        new_hash: ObjectHash,
        similarity: u8,
    ) -> Result<DiffItem, GitError> {
        let mut patch = Self::format_relocated_patch_header(old_path, new_path, similarity);

        if old_hash != new_hash {
            let raw_items = self
                .get_diff_by_blobs(
                    vec![(old_path.to_path_buf(), old_hash)],
                    vec![(old_path.to_path_buf(), new_hash)],
                )
                .await?;
            if let Some(item) = raw_items.into_iter().next() {
                patch.push_str(&Self::relocate_patch_body(&item.data, old_path, new_path));
            }
        }

        Ok(DiffItem {
            path: new_path.to_string_lossy().replace('\\', "/"),
            data: patch,
        })
    }

    fn format_relocated_patch_header(old_path: &Path, new_path: &Path, similarity: u8) -> String {
        let old_path = old_path.to_string_lossy().replace('\\', "/");
        let new_path = new_path.to_string_lossy().replace('\\', "/");
        format!(
            "diff --git a/{old_path} b/{new_path}\nsimilarity index {similarity}%\nrename from {old_path}\nrename to {new_path}\n"
        )
    }

    pub(crate) fn normalize_diff_item(mut item: DiffItem) -> DiffItem {
        item.path = item.path.replace('\\', "/");
        item.data = Self::normalize_patch_header_paths(&item.data);
        item
    }

    pub(crate) fn normalize_patch_header_paths(raw_patch: &str) -> String {
        let sections = Self::split_patch_sections(raw_patch);
        let header_lines = sections
            .header_lines
            .into_iter()
            .map(Self::normalize_patch_header_line)
            .collect();
        let divider_line = sections.divider_line.map(|line| {
            if line.starts_with("Binary files ") {
                line.replace('\\', "/")
            } else {
                line.to_string()
            }
        });

        Self::join_patch_sections(
            header_lines,
            divider_line,
            sections.payload_lines,
            sections.has_trailing_newline,
        )
    }

    pub(crate) fn relocate_patch_body(raw_patch: &str, old_path: &Path, new_path: &Path) -> String {
        let old_path = old_path.to_string_lossy().replace('\\', "/");
        let new_path = new_path.to_string_lossy().replace('\\', "/");
        let sections = Self::split_patch_sections(raw_patch);
        let header_lines = sections
            .header_lines
            .into_iter()
            .filter(|line| !line.starts_with("diff --git "))
            .map(|line| {
                if line.starts_with("--- a/") {
                    format!("--- a/{old_path}")
                } else if line.starts_with("+++ b/") {
                    format!("+++ b/{new_path}")
                } else {
                    line.to_string()
                }
            })
            .collect();
        let divider_line = sections.divider_line.map(|line| {
            if line.starts_with("Binary files ") {
                format!("Binary files a/{old_path} and b/{new_path} differ")
            } else {
                line.to_string()
            }
        });

        Self::join_patch_sections(
            header_lines,
            divider_line,
            sections.payload_lines,
            sections.has_trailing_newline,
        )
    }

    fn normalize_patch_header_line(line: &str) -> String {
        if line.starts_with("diff --git ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("rename from ")
            || line.starts_with("rename to ")
        {
            line.replace('\\', "/")
        } else {
            line.to_string()
        }
    }

    fn split_patch_sections(raw_patch: &str) -> PatchSections<'_> {
        let mut header_lines = Vec::new();
        let mut divider_line = None;
        let mut payload_lines = Vec::new();
        let mut in_payload = false;

        for line in raw_patch.lines() {
            if in_payload {
                payload_lines.push(line);
                continue;
            }

            if line.starts_with("@@")
                || line.starts_with("Binary files ")
                || line.starts_with("GIT binary patch")
            {
                divider_line = Some(line);
                in_payload = true;
                continue;
            }

            header_lines.push(line);
        }

        PatchSections {
            header_lines,
            divider_line,
            payload_lines,
            has_trailing_newline: raw_patch.ends_with('\n'),
        }
    }

    fn join_patch_sections(
        header_lines: Vec<String>,
        divider_line: Option<String>,
        payload_lines: Vec<&str>,
        has_trailing_newline: bool,
    ) -> String {
        let mut lines = header_lines;
        if let Some(line) = divider_line {
            lines.push(line);
        }
        lines.extend(payload_lines.into_iter().map(String::from));

        let rendered = lines.join("\n");
        if rendered.is_empty() {
            rendered
        } else if has_trailing_newline {
            format!("{rendered}\n")
        } else {
            rendered
        }
    }

    pub async fn get_sorted_changed_file_list(
        &self,
        cl_link: &str,
        path: Option<&str>,
    ) -> Result<Vec<String>, MegaError> {
        let normalized_prefix = path.map(|prefix| prefix.replace('\\', "/"));
        let cl = self
            .storage()
            .cl_service
            .cl_store()
            .get_cl(cl_link)
            .await
            .unwrap()
            .ok_or_else(|| MegaError::Other("Error getting ".to_string()))?;

        let old_files = self.get_commit_blobs(&cl.from_hash.clone()).await?;
        let new_files = self.get_commit_blobs(&cl.to_hash.clone()).await?;

        // calculate pages
        let sorted_changed_files = self.cl_files_list(old_files, new_files).await?;
        let file_paths: Vec<String> = sorted_changed_files
            .iter()
            .map(|f| f.path().to_string_lossy().replace('\\', "/"))
            .filter(|file_path| {
                if let Some(prefix) = &normalized_prefix {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .collect();

        Ok(file_paths)
    }
    pub async fn cl_files_list(
        &self,
        old_files: Vec<(PathBuf, ObjectHash)>,
        new_files: Vec<(PathBuf, ObjectHash)>,
    ) -> Result<Vec<ClDiffFile>, MegaError> {
        let base_diff = tree_diff::calculate_tree_diff_basic(old_files.clone(), new_files.clone())?;
        let mut blob_cache: HashMap<ObjectHash, Vec<u8>> = HashMap::new();
        let mut failed_hashes = Vec::new();
        let candidate_hashes: HashSet<ObjectHash> = base_diff
            .iter()
            .filter_map(|item| match item {
                ClDiffFile::Deleted(_, hash) | ClDiffFile::New(_, hash) => Some(*hash),
                _ => None,
            })
            .collect();

        if base_diff.len() > LARGE_CL_RENAME_DETECTION_THRESHOLD
            || candidate_hashes.len() > LARGE_CL_RENAME_DETECTION_THRESHOLD
        {
            tracing::info!(
                diff_files = base_diff.len(),
                candidate_hashes = candidate_hashes.len(),
                threshold = LARGE_CL_RENAME_DETECTION_THRESHOLD,
                "Skipping rename detection for large CL diff and returning path-level results."
            );
            return Ok(base_diff);
        }

        for hash in candidate_hashes {
            match self.git().get_raw_blob_by_hash(&hash.to_string()).await {
                Ok(data) => {
                    blob_cache.insert(hash, data);
                }
                Err(err) => {
                    failed_hashes.push(hash);
                    tracing::warn!(
                        "rename detection skipped blob {} and will fall back to path-level diff: {}",
                        hash,
                        err
                    );
                }
            }
        }

        if !failed_hashes.is_empty() {
            tracing::warn!(
                "rename detection degraded for {} candidate blob(s)",
                failed_hashes.len()
            );
        }

        let rename_config = self.storage().config().monorepo.rename.clone();
        tree_diff::calculate_tree_diff_with_blobs(old_files, new_files, &rename_config, &blob_cache)
    }

    pub async fn get_commit_blobs(
        &self,
        commit_hash: &str,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        let mut res = vec![];
        let mono_storage = self.storage().mono_storage();
        let commit = mono_storage.get_commit_by_hash(commit_hash).await?;
        if let Some(commit) = commit {
            let tree = mono_storage.get_tree_by_hash(&commit.tree).await?;
            if let Some(tree) = tree {
                let tree: Tree = Tree::from_mega_model(tree);
                res = self.git_ops().traverse_tree(tree).await?;
            }
        }
        Ok(res)
    }
}

pub(crate) fn collect_page_blobs(
    items: &[ClDiffFile],
    old_out: &mut Vec<(PathBuf, ObjectHash)>,
    new_out: &mut Vec<(PathBuf, ObjectHash)>,
) {
    old_out.reserve(items.len());
    new_out.reserve(items.len());

    for item in items {
        match item {
            ClDiffFile::New(p, h_new) => {
                new_out.push((p.clone(), *h_new));
            }
            ClDiffFile::Deleted(p, h_old) => {
                old_out.push((p.clone(), *h_old));
            }
            ClDiffFile::Modified(p, h_old, h_new) => {
                old_out.push((p.clone(), *h_old));
                new_out.push((p.clone(), *h_new));
            }
            ClDiffFile::Renamed(_, _, _, _, _) | ClDiffFile::Moved(_, _, _, _, _) => {
                // Relocated items are filtered out before this helper is called.
                debug_assert!(false, "collect_page_blobs only accepts non-relocated items");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        str::FromStr,
    };

    use git_internal::{DiffItem, hash::ObjectHash};

    use super::collect_page_blobs;
    use crate::{
        application::api_service::mono::ClApplicationService, model::change_list::ClDiffFile,
    };

    #[test]
    fn test_paging_calculation_basic() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
                ObjectHash::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
    }

    #[test]
    fn test_paging_calculation_second_page() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
                ObjectHash::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
            ClDiffFile::New(
                PathBuf::from("file4.txt"),
                ObjectHash::from_str("2222222222222222222222222222222222222222").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 2u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 2);
        assert_eq!(end, 4);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
        assert_eq!(page_slice[0].path(), &PathBuf::from("file3.txt"));
        assert_eq!(page_slice[1].path(), &PathBuf::from("file4.txt"));
    }

    #[test]
    fn test_paging_calculation_partial_page() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
                ObjectHash::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("file3.txt"),
                ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
        ];

        let page_size = 5u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 3);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 3);
    }

    #[test]
    fn test_paging_calculation_out_of_bounds() {
        let files: Vec<ClDiffFile> = vec![ClDiffFile::New(
            PathBuf::from("file1.txt"),
            ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let page_size = 2u32;
        let page_id = 3u32; // Page that doesn't exist

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 4);
        assert_eq!(end, 1); // end is clamped to files.len()

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 0);
    }

    #[test]
    fn test_paging_calculation_edge_case_zero_page_size() {
        let files: Vec<ClDiffFile> = vec![ClDiffFile::New(
            PathBuf::from("file1.txt"),
            ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let page_size = 0u32;
        let page_id = 1u32;

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 0);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 0);
    }

    #[test]
    fn test_paging_calculation_zero_page_id() {
        let files: Vec<ClDiffFile> = vec![
            ClDiffFile::New(
                PathBuf::from("file1.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("file2.txt"),
                ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
                ObjectHash::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            ),
        ];

        let page_size = 2u32;
        let page_id = 0u32; // Should be treated as page 1 due to saturating_sub

        let start = (page_id.saturating_sub(1)) * page_size;
        let end = (start + page_size).min(files.len() as u32);

        assert_eq!(start, 0);
        assert_eq!(end, 2);

        let page_slice: &[ClDiffFile] = if (start as usize) < files.len() {
            let start_idx = start as usize;
            let end_idx = end as usize;
            &files[start_idx..end_idx]
        } else {
            &[]
        };

        assert_eq!(page_slice.len(), 2);
    }

    #[test]
    fn test_paging_algorithm() {
        let total_files = 10usize;
        let current_page = 2u32;
        let page_size = 3u32;

        let total_pages = total_files.div_ceil(page_size as usize);
        let current_page = current_page as usize;
        let page_size = page_size as usize;

        assert_eq!(total_pages, 4);
        assert_eq!(current_page, 2);
        assert_eq!(page_size, 3);
    }

    #[test]
    fn test_collect_page_blobs_new_files() {
        let files = vec![ClDiffFile::New(
            PathBuf::from("new_file.txt"),
            ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 0);
        assert_eq!(new_blobs.len(), 1);
        assert_eq!(new_blobs[0].0, PathBuf::from("new_file.txt"));
    }

    #[test]
    fn test_collect_page_blobs_deleted_files() {
        let files = vec![ClDiffFile::Deleted(
            PathBuf::from("deleted_file.txt"),
            ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 1);
        assert_eq!(new_blobs.len(), 0);
        assert_eq!(old_blobs[0].0, PathBuf::from("deleted_file.txt"));
    }

    #[test]
    fn test_file_lists_with_roots() {
        let all_files = vec![
            "src/main.rs".to_string(),
            "src/utils/math.rs".to_string(),
            "src/utils/io.rs".to_string(),
            "README.md".to_string(),
        ];

        let root: Option<&str> = None;
        let filtered_none: Vec<String> = all_files
            .iter()
            .filter(|file_path| {
                if let Some(prefix) = root {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        assert_eq!(filtered_none.len(), 4);
        assert_eq!(filtered_none, all_files);

        let filtered_some: Vec<String> = all_files
            .iter()
            .filter(|file_path| {
                if let Some(prefix) = Some("src/utils") {
                    file_path.starts_with(prefix)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        assert_eq!(filtered_some.len(), 2);
        assert_eq!(
            filtered_some,
            vec![
                "src/utils/math.rs".to_string(),
                "src/utils/io.rs".to_string()
            ]
        );
    }

    #[test]
    fn test_collect_page_blobs_modified_files() {
        let files = vec![ClDiffFile::Modified(
            PathBuf::from("modified_file.txt"),
            ObjectHash::from_str("1234567890123456789012345678901234567890").unwrap(),
            ObjectHash::from_str("abcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
        )];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 1);
        assert_eq!(new_blobs.len(), 1);
        assert_eq!(old_blobs[0].0, PathBuf::from("modified_file.txt"));
        assert_eq!(new_blobs[0].0, PathBuf::from("modified_file.txt"));
    }

    #[test]
    fn test_collect_page_blobs_mixed_files() {
        let files = vec![
            ClDiffFile::New(
                PathBuf::from("new.txt"),
                ObjectHash::from_str("1111111111111111111111111111111111111111").unwrap(),
            ),
            ClDiffFile::Deleted(
                PathBuf::from("deleted.txt"),
                ObjectHash::from_str("2222222222222222222222222222222222222222").unwrap(),
            ),
            ClDiffFile::Modified(
                PathBuf::from("modified.txt"),
                ObjectHash::from_str("3333333333333333333333333333333333333333").unwrap(),
                ObjectHash::from_str("4444444444444444444444444444444444444444").unwrap(),
            ),
        ];

        let mut old_blobs = Vec::new();
        let mut new_blobs = Vec::new();

        collect_page_blobs(&files, &mut old_blobs, &mut new_blobs);

        assert_eq!(old_blobs.len(), 2); // deleted + modified
        assert_eq!(new_blobs.len(), 2); // new + modified

        assert_eq!(old_blobs[0].0, PathBuf::from("deleted.txt"));
        assert_eq!(old_blobs[1].0, PathBuf::from("modified.txt"));
        assert_eq!(new_blobs[0].0, PathBuf::from("new.txt"));
        assert_eq!(new_blobs[1].0, PathBuf::from("modified.txt"));
    }

    #[test]
    fn test_relocate_patch_body_rewrites_paths_and_keeps_hunk() {
        let raw_patch = "\
diff --git a/old/name.txt b/old/name.txt\n\
index 1111111..2222222 100644\n\
--- a/old/name.txt\n\
+++ b/old/name.txt\n\
@@ -1 +1 @@\n\
-old line\n\
+new line\n";

        let relocated = ClApplicationService::relocate_patch_body(
            raw_patch,
            Path::new("old/name.txt"),
            Path::new("new/name.txt"),
        );

        assert!(!relocated.contains("diff --git"));
        assert!(relocated.contains("--- a/old/name.txt"));
        assert!(relocated.contains("+++ b/new/name.txt"));
        assert!(relocated.contains("@@ -1 +1 @@"));
        assert!(relocated.contains("-old line"));
        assert!(relocated.contains("+new line"));
        assert!(!relocated.contains("deleted file mode"));
    }

    #[test]
    fn test_relocate_patch_body_preserves_hunk_backslashes() {
        let raw_patch = "\
diff --git a/old/name.txt b/old/name.txt\n\
index 1111111..2222222 100644\n\
--- a/old/name.txt\n\
+++ b/old/name.txt\n\
@@ -1 +1 @@\n\
-let path = \"C:\\\\temp\\\\old\";\n\
+let path = \"C:\\\\temp\\\\new\";\n";

        let relocated = ClApplicationService::relocate_patch_body(
            raw_patch,
            Path::new("old/name.txt"),
            Path::new("new/name.txt"),
        );

        assert!(relocated.contains("--- a/old/name.txt"));
        assert!(relocated.contains("+++ b/new/name.txt"));
        assert!(relocated.contains("-let path = \"C:\\\\temp\\\\old\";"));
        assert!(relocated.contains("+let path = \"C:\\\\temp\\\\new\";"));
    }

    #[test]
    fn test_normalize_diff_item_path_uses_forward_slashes() {
        let item = DiffItem {
            path: "dir\\nested\\file.txt".to_string(),
            data: "diff --git a/dir\\nested\\file.txt b/dir\\nested\\file.txt\n".to_string(),
        };

        let normalized = ClApplicationService::normalize_diff_item(item);
        assert_eq!(normalized.path, "dir/nested/file.txt");
        assert!(
            normalized
                .data
                .contains("diff --git a/dir/nested/file.txt b/dir/nested/file.txt")
        );
    }

    #[test]
    fn test_normalize_patch_header_paths_preserves_hunk_content() {
        let raw_patch = "\
diff --git a/dir\\nested\\file.txt b/dir\\nested\\file.txt\n\
--- a/dir\\nested\\file.txt\n\
+++ b/dir\\nested\\file.txt\n\
@@ -1 +1 @@\n\
-let path = \"C:\\\\temp\\\\old\";\n\
+let path = \"C:\\\\temp\\\\new\";\n";

        let normalized = ClApplicationService::normalize_patch_header_paths(raw_patch);

        assert!(normalized.contains("diff --git a/dir/nested/file.txt b/dir/nested/file.txt"));
        assert!(normalized.contains("--- a/dir/nested/file.txt"));
        assert!(normalized.contains("+++ b/dir/nested/file.txt"));
        assert!(normalized.contains("-let path = \"C:\\\\temp\\\\old\";"));
        assert!(normalized.contains("+let path = \"C:\\\\temp\\\\new\";"));
    }
}
