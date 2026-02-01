use sha2::{Digest, Sha256};

pub struct DiffHunk {
    pub start_original: usize,
    pub num_original: usize,
    pub start_new: usize,
    pub num_new: usize,
    pub lines: Vec<String>,
}

pub fn hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<String>()
}

/// Checks whether the surrounding context of a candidate line position
/// is sufficiently similar to the original anchor context.
///
/// The function compares the normalized previous and/or next lines against
/// the stored normalized context using a similarity threshold.
/// If either side matches well enough, the position is considered valid.
///
/// This is intended as a tolerant fallback during anchor re-matching,
/// allowing minor edits while preserving logical proximity.
pub fn context_match(lines: &[&str], center: isize, before_norm: &str, after_norm: &str) -> bool {
    let mut before_ok = true;
    let mut after_ok = true;

    if !before_norm.is_empty() && center > 0 {
        let prev = normalize(lines[(center - 1) as usize]);
        before_ok = similar_score(&prev, before_norm) >= 0.8;
    }

    if !after_norm.is_empty() && (center as usize + 1) < lines.len() {
        let next = normalize(lines[(center + 1) as usize]);
        after_ok = similar_score(&next, after_norm) >= 0.8;
    }

    before_ok || after_ok
}

/// Determines whether two normalized strings are similar enough
/// based on the ratio of their Longest Common Subsequence (LCS).
///
/// Similarity is defined as:
///     LCS length / max(input length)
pub fn similar_score(a: &str, b: &str) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let lcs = lcs_len(a, b) as f32;
    let max_len = a.len().max(b.len()) as f32;

    lcs / max_len
}

/// Computes the length of the Longest Common Subsequence (LCS)
/// between two strings at the byte level.
fn lcs_len(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];

    for i in 0..a.len() {
        for j in 0..b.len() {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    dp[a.len()][b.len()]
}

/// Parses a unified diff string (such as produced by `git diff`) into a list of hunks.
///
/// A "hunk" represents a contiguous block of changes in a file, including context lines,
/// additions, and deletions. Each hunk contains information about:
/// - The starting line number and number of lines in the original file (`start_original`, `num_original`)
/// - The starting line number and number of lines in the new file (`start_new`, `num_new`)
/// - The actual lines of the hunk, with each line typically starting with:
///     - `'+'` for added lines
///     - `'-'` for removed lines
///     - `' '` for unchanged context lines
///
/// # Parameters
/// - `diff`: The input unified diff as a string slice.
///
/// # Returns
/// A vector of `DiffHunk`, each representing a parsed hunk from the diff.
///
/// # Notes
/// - Lines before the first hunk header (e.g., `diff --git ...`, `index ...`, `---`, `+++`) are ignored.
/// - The function assumes the diff is in a valid unified diff format; malformed headers may cause panics.
/// - Multiple hunks per file are fully supported.
/// - Lines starting with multiple `-` characters (e.g., `--`) are treated as deleted lines, not hunk headers.
pub fn parse_unified_diff(diff: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    for file_chunk in diff.split("diff --git ").skip(1) {
        if file_chunk.contains("deleted file mode") || file_chunk.contains("Binary files") {
            continue;
        }

        let mut lines = file_chunk.lines().peekable();

        while let Some(line) = lines.next() {
            if line.starts_with("@@") {
                let parts: Vec<_> = line.split_whitespace().collect();
                let orig_range = &parts[1];
                let new_range = &parts[2];

                let parse_range = |s: &str| -> (usize, usize) {
                    let s = s.trim_start_matches(['-', '+']);
                    let mut it = s.split(',');
                    let start = it.next().unwrap().parse::<usize>().unwrap();
                    let count = it.next().unwrap_or("1").parse::<usize>().unwrap();
                    (start, count)
                };

                let (start_original, num_original) = parse_range(orig_range);
                let (start_new, num_new) = parse_range(new_range);

                let mut hunk_lines = Vec::new();
                while let Some(&l) = lines.peek() {
                    if l.starts_with("@@") {
                        break;
                    }
                    hunk_lines.push(lines.next().unwrap().to_string());
                }

                hunks.push(DiffHunk {
                    start_original,
                    num_original,
                    start_new,
                    num_new,
                    lines: hunk_lines,
                });
            }
        }
    }

    hunks
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_unified_diff() {
        let diff_data = r#"diff --git a/libra/src/internal/model/config.rs b/libra/src/internal/model/config.rs
index 14e6e59..a1b2c3d 100644
--- a/libra/src/internal/model/config.rs
+++ b/libra/src/internal/model/config.rs
@@ -5,6 +5,9 @@
 use sea_orm::entity::prelude::*;
 
 #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
+#[sea_orm(table_name = "config")]
 pub struct Model {
     #[sea_orm(primary_key, auto_increment = true)]
     pub id: i64,
+    pub configuration: String,
+    pub name: Option<String>,
     pub key: String,
     pub value: String,
 }
@@ -15,6 +18,9 @@
 #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
 pub enum Relation {}
 
+impl ActiveModelBehavior for ActiveModel {}
+
+// Added new helper struct
+pub struct Helper {
+    pub description: String,
+}
"#;

        let hunks = parse_unified_diff(diff_data);
        assert_eq!(hunks.len(), 2);

        let hunk1 = &hunks[0];
        assert_eq!(hunk1.start_original, 5);
        assert_eq!(hunk1.num_original, 6);
        assert_eq!(hunk1.start_new, 5);
        assert_eq!(hunk1.num_new, 9);
        assert!(
            hunk1
                .lines
                .iter()
                .any(|l| l.starts_with("+#[sea_orm(table_name = \"config\")]"))
        );

        let hunk2 = &hunks[1];
        assert_eq!(hunk2.start_original, 15);
        assert_eq!(hunk2.num_original, 6);
        assert_eq!(hunk2.start_new, 18);
        assert_eq!(hunk2.num_new, 9);
        assert!(
            hunk2
                .lines
                .iter()
                .any(|l| l.starts_with("+impl ActiveModelBehavior"))
        );
        assert!(
            hunk2
                .lines
                .iter()
                .any(|l| l.starts_with("+pub struct Helper"))
        );
    }
}
