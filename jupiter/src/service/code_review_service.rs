use std::{collections::HashMap, vec};

use callisto::{
    mega_code_review_anchor, mega_code_review_comment, mega_code_review_position,
    mega_code_review_thread,
    sea_orm_active_enums::{DiffSideEnum, PositionStatusEnum, ThreadStatusEnum},
};
use common::errors::MegaError;
use git_internal::DiffItem;

use crate::{
    model::code_review_dto::{
        CodeReviewViews, CommentReviewView, FileReviewView, ThreadReviewView,
    },
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        code_review_comment_storage::CodeReviewCommentStorage,
        code_review_thread_storage::CodeReviewThreadStorage,
    },
    utils::code_review_reanchor::{
        context_match, hash, normalize, parse_unified_diff, similar_score,
    },
};

#[derive(Clone)]
pub struct CodeReviewService {
    pub code_review_thread: CodeReviewThreadStorage,
    pub code_review_comment: CodeReviewCommentStorage,
}

impl CodeReviewService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            code_review_thread: CodeReviewThreadStorage {
                base: base_storage.clone(),
            },
            code_review_comment: CodeReviewCommentStorage {
                base: base_storage.clone(),
            },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            code_review_thread: CodeReviewThreadStorage { base: mock.clone() },
            code_review_comment: CodeReviewCommentStorage { base: mock.clone() },
        }
    }

    pub async fn get_all_comments_by_link(&self, link: &str) -> Result<CodeReviewViews, MegaError> {
        let threads = self
            .code_review_thread
            .get_code_review_threads_by_link(link)
            .await?;

        if threads.is_empty() {
            return Ok(CodeReviewViews {
                link: link.to_string(),
                files: vec![],
            });
        }

        let thread_ids: Vec<i64> = threads.iter().map(|t| t.id).collect();

        // Batch fetch related entities
        let anchors = self
            .code_review_thread
            .get_anchors_by_thread_ids(&thread_ids)
            .await?;
        let positions = self
            .code_review_thread
            .get_positions_by_thread_ids(&thread_ids)
            .await?;
        let comments = self
            .code_review_comment
            .get_comments_by_thread_ids(&thread_ids)
            .await?;

        // Map entities by thread_id or anchor_id
        let comments_by_thread: HashMap<i64, Vec<_>> =
            comments.into_iter().fold(HashMap::new(), |mut map, c| {
                map.entry(c.thread_id).or_default().push(c);
                map
            });

        let anchors_by_thread: HashMap<i64, Vec<_>> =
            anchors.into_iter().fold(HashMap::new(), |mut map, a| {
                map.entry(a.thread_id).or_default().push(a);
                map
            });

        let positions_by_anchor: HashMap<i64, _> =
            positions.into_iter().map(|p| (p.anchor_id, p)).collect();

        // Build ThreadReviewView
        let mut files_map: HashMap<String, Vec<ThreadReviewView>> = HashMap::new();

        for thread in &threads {
            if let Some(thread_anchors) = anchors_by_thread.get(&thread.id) {
                for anchor in thread_anchors {
                    let position = positions_by_anchor.get(&anchor.id).ok_or_else(|| {
                        MegaError::Other(format!("Position not found for anchor {}", anchor.id))
                    })?;

                    let thread_comments = comments_by_thread
                        .get(&thread.id)
                        .cloned()
                        .unwrap_or_default();

                    let thread_view = ThreadReviewView::from_models(
                        thread.clone(),
                        anchor.clone(),
                        position.clone(),
                        thread_comments,
                    );

                    files_map
                        .entry(anchor.file_path.clone())
                        .or_default()
                        .push(thread_view);
                }
            }
        }

        let files = files_map
            .into_iter()
            .map(|(file_path, threads)| FileReviewView { file_path, threads })
            .collect();

        Ok(CodeReviewViews {
            link: link.to_string(),
            files,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_inline_comment(
        &self,
        link: &str,
        file_path: &str,
        diff_side: DiffSideEnum,
        anchor_commit_sha: &str,
        original_line_number: i32,
        normalized_content: &str,
        context_before: &str,
        context_after: &str,
        user_name: String,
        content: String,
    ) -> Result<ThreadReviewView, MegaError> {
        let (thread, anchor, position) = self
            .code_review_thread
            .create_thread_by_anchor(
                link,
                file_path,
                &diff_side,
                anchor_commit_sha,
                original_line_number,
                normalized_content,
                context_before,
                context_after,
            )
            .await?;

        let comment = self
            .code_review_comment
            .create_code_review_comment(thread.id, user_name, None, Some(content))
            .await?;

        let thread = self.code_review_thread.touch_thread(thread.id).await?;

        Ok(ThreadReviewView::from_models(
            thread,
            anchor,
            position,
            vec![comment],
        ))
    }

    pub async fn reply_to_comment(
        &self,
        thread_id: i64,
        parent_comment_id: i64,
        user_name: String,
        content: String,
    ) -> Result<CommentReviewView, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let parent_comment = self
            .code_review_comment
            .find_comment_by_id(parent_comment_id)
            .await?
            .ok_or_else(|| {
                MegaError::Other(format!("Parent comment {} not found", parent_comment_id))
            })?;

        if parent_comment.thread_id != thread_id {
            return Err(MegaError::Other(
                "Parent comment does not belong to the thread".to_string(),
            ));
        }

        let comment = self
            .code_review_comment
            .create_code_review_comment(
                thread_id,
                user_name,
                Some(parent_comment_id),
                Some(content),
            )
            .await?;

        Ok(comment.into())
    }

    pub async fn update_comment(
        &self,
        comment_id: i64,
        new_content: String,
    ) -> Result<CommentReviewView, MegaError> {
        self.code_review_comment
            .find_comment_by_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        let updated_comment = self
            .code_review_comment
            .update_code_review_comment(comment_id, Some(new_content))
            .await?;

        Ok(updated_comment.into())
    }

    pub async fn resolve_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let updated_thread = self
            .code_review_thread
            .update_code_review_thread_status(thread_id, ThreadStatusEnum::Resolved)
            .await?;

        Ok(updated_thread)
    }

    pub async fn reopen_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        self.code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        let updated_thread = self
            .code_review_thread
            .update_code_review_thread_status(thread_id, ThreadStatusEnum::Open)
            .await?;

        Ok(updated_thread)
    }

    pub async fn delete_thread(
        &self,
        thread_id: i64,
    ) -> Result<mega_code_review_thread::Model, MegaError> {
        let thread = self
            .code_review_thread
            .find_thread_by_id(thread_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Thread {} not found", thread_id)))?;

        self.code_review_comment
            .delete_comments_by_thread_id(thread_id)
            .await?;
        self.code_review_thread
            .delete_code_review_thread(thread_id)
            .await?;

        Ok(thread)
    }

    pub async fn delete_comment(
        &self,
        comment_id: i64,
    ) -> Result<mega_code_review_comment::Model, MegaError> {
        let comment = self
            .code_review_comment
            .find_comment_by_id(comment_id)
            .await?
            .ok_or_else(|| MegaError::Other(format!("Comment {} not found", comment_id)))?;

        self.code_review_comment
            .delete_comment_by_comment_id(comment_id)
            .await?;
        Ok(comment)
    }

    pub async fn reanchor_thread(
        &self,
        anchor: &mega_code_review_anchor::Model,
        latest_blob: Option<String>,
        diff_items: Vec<DiffItem>,
        current_commit_sha: &str,
    ) -> Result<mega_code_review_position::Model, MegaError> {
        let Some(latest_blob) = latest_blob else {
            return Err(MegaError::Other("latest blob missing".to_string()));
        };

        let latest_lines: Vec<&str> = latest_blob.lines().collect();

        // Tier 1: absolute line number No Change
        if let Some(line_number) = try_tier1_no_change(anchor, &latest_lines) {
            return self
                .code_review_thread
                .update_position(
                    anchor.id,
                    current_commit_sha,
                    Some(line_number),
                    100,
                    PositionStatusEnum::Exact,
                )
                .await;
        }

        // Tier 2: diff hunk line number shift
        if let Some((line_number, position_status, score)) =
            try_tier2_diff_hunk_shift(anchor, diff_items)
        {
            return self
                .code_review_thread
                .update_position(
                    anchor.id,
                    current_commit_sha,
                    Some(line_number),
                    score as i32,
                    position_status,
                )
                .await;
        }

        // Tier 3(todo): Full document coverage

        // Not Found
        self.code_review_thread
            .update_position(
                anchor.id,
                current_commit_sha,
                None,
                0,
                PositionStatusEnum::NotFound,
            )
            .await
    }
}

/// Tier 1 re-anchoring: "No Change" validation.
///
/// This function verifies that a newly pushed commit does NOT affect
/// the original commented line at all.
///
/// A Tier 1 success means:
/// - The absolute line number remains unchanged
/// - The normalized content hash is identical
/// - The surrounding context structure is still valid
///
/// If all checks pass, the comment position is considered stable and
/// no re-anchoring or line number adjustment is required.
///
/// Returns:
/// - Some(original_line_number) if the anchor is still valid
/// - None if Tier 1 conditions are not satisfied (should fall back to Tier 2)
fn try_tier1_no_change(
    anchor: &mega_code_review_anchor::Model,
    latest_lines: &[&str],
) -> Option<i32> {
    let idx = anchor.original_line_number as isize - 1;

    // Abort if the original line number is out of bounds
    if idx < 0 || idx as usize >= latest_lines.len() {
        return None;
    }

    let line = latest_lines[idx as usize];
    let normalized = normalize(line);

    // Strict content verification:
    // The normalized content hash or line content hash must match exactly
    if hash(&normalized) != anchor.normalized_hash || hash(line) != anchor.normalized_hash {
        return None;
    }

    // Context validation:
    // Prevents false positives caused by identical lines elsewhere in the file
    if !context_match(
        latest_lines,
        idx,
        &anchor.context_before_hash,
        &anchor.context_after_hash,
    ) {
        return None;
    }

    // Tier 1 success: The comment remains anchored to the same absolute line number
    Some(anchor.original_line_number)
}

/// Tier 2 re-anchoring: Diff hunk line number shift.
///
/// This tier handles cases where the original commented line itself
/// remains unchanged in content, but its absolute line number has shifted
/// due to insertions or deletions elsewhere in the same diff hunk.
///
/// A Tier 2 success means:
/// - The line content (after normalization) is identical to the original
/// - The original line can be relocated by applying a diff-based offset
/// - The change is limited to structural shifts, not semantic edits
///
/// This tier assumes that the comment target is still the same logical line,
/// and only requires a line number adjustment derived from the diff.
///
/// If this tier succeeds, the comment position is updated with the new
/// line number, but retains a high confidence level.
///
/// Returns:
/// - Some(new_line_number) if the line is found via diff hunk adjustment
/// - None if the line cannot be reliably relocated (should fall back to Tier 3)
pub fn try_tier2_diff_hunk_shift(
    anchor: &mega_code_review_anchor::Model,
    diff_items: Vec<DiffItem>,
) -> Option<(i32, PositionStatusEnum, u32)> {
    let diff_item = diff_items
        .iter()
        .find(|item| item.path == anchor.file_path)?;

    // Parse unified diff into hunks
    let hunks = parse_unified_diff(&diff_item.data);

    for hunk in hunks {
        let orig_start = hunk.start_original as i32;
        let orig_end = orig_start + hunk.num_original as i32 - 1;

        // Check if the original line is within this hunk
        if !(orig_start..=orig_end).contains(&(anchor.original_line_number)) {
            continue;
        }

        // Initialize line tracking variables
        let mut orig_line = orig_start; // current original line number
        let mut new_line = hunk.start_new as i32; // current new line number
        let mut line_offset = 0;

        // Iterate through hunk lines once
        for line in &hunk.lines {
            match line.chars().next() {
                Some('+') => {
                    // Added line: shift anchor line if it is after current original line
                    if orig_line <= anchor.original_line_number {
                        line_offset += 1;
                    }
                    // '+' lines do not advance original line
                    if orig_line <= anchor.original_line_number {
                        // new_line increments for added line
                        new_line += 1;
                    }
                }
                Some('-') => {
                    // Deleted line: shift anchor line if it is after current original line
                    if orig_line < anchor.original_line_number {
                        line_offset -= 1;
                    } else if orig_line == anchor.original_line_number {
                        // Original line was deleted; cannot relocate in Tier 2
                        return None;
                    }
                    // Deleted line does not advance new_line
                    orig_line += 1;
                    continue;
                }
                Some(' ') | None => {
                    // Context line: advance both original and new line numbers
                    orig_line += 1;
                    new_line += 1;
                }
                _ => {}
            }

            // Check if the current new line matches the anchor
            let target_line_number = anchor.original_line_number + line_offset;
            if new_line == target_line_number {
                let content = line.trim_start_matches(['+', ' ']).to_string();
                let score: u32 =
                    (similar_score(&normalize(&content), &normalize(&anchor.normalized_content))
                        * 100.0)
                        .round() as u32;

                if score >= 90 {
                    return Some((target_line_number, PositionStatusEnum::Shifted, score));
                } else if score >= 60 {
                    return Some((target_line_number, PositionStatusEnum::Ambiguous, score));
                } else {
                    return None;
                }
            }
        }
    }

    // Not found in any hunk, Tier 2 fails
    None
}
