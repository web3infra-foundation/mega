use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use bytes::Bytes;
use common::{
    errors::{BuckError, MegaError},
    utils::ZERO_ID,
};
use regex::Regex;

use super::MonoServiceLogic;

static PATH_NOT_EXIST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Path '([^']+)' not exist").expect("PATH_NOT_EXIST_RE must be valid")
});

pub(crate) fn path_not_exist_re() -> &'static Regex {
    &PATH_NOT_EXIST_RE
}

impl MonoServiceLogic {
    pub fn clean_path_str(path: &str) -> String {
        let s = path.trim_end_matches('/');
        if s.is_empty() {
            "/".to_string()
        } else {
            s.to_string()
        }
    }

    /// Normalize and validate repository path.
    ///
    /// Rules: trim; reject empty or whitespace-only (validation error). Reject `..`, backslash,
    /// Windows drive letters (e.g. `C:`), and paths starting with `:`. Strip trailing `/`;
    /// input consisting only of slashes becomes `"/"`. Collapse middle repeated slashes and
    /// remove `.` segments (e.g. `//project//foo` -> `/project/foo`, `project/./foo` -> `/project/foo`).
    /// Paths that consist only of `.` and slashes (e.g. `"."`, `"./"`) are rejected so they do not
    /// silently resolve to root. Non-empty result gets a leading `"/"` if missing. Result matches
    /// mega_refs.path format.
    pub fn normalize_repo_path(path: &str) -> Result<String, MegaError> {
        let s = path.trim();
        if s.is_empty() {
            return Err(MegaError::Buck(BuckError::ValidationError(
                "Path cannot be empty".to_string(),
            )));
        }
        if s.contains("..") {
            return Err(MegaError::Buck(BuckError::ValidationError(format!(
                "Path traversal not allowed: {}",
                s
            ))));
        }
        if s.contains('\\') {
            return Err(MegaError::Buck(BuckError::ValidationError(format!(
                "Path must use '/' separator: {}",
                s
            ))));
        }
        if s.len() >= 2 {
            let mut chars = s.chars();
            if let (Some(c1), Some(':')) = (chars.next(), chars.next())
                && c1.is_ascii_alphabetic()
            {
                return Err(MegaError::Buck(BuckError::ValidationError(format!(
                    "Absolute path not allowed (Windows drive letter detected): {}",
                    s
                ))));
            }
        }
        if s.starts_with(':') {
            return Err(MegaError::Buck(BuckError::ValidationError(
                "Path must not start with ':'".to_string(),
            )));
        }
        let s = s.trim_end_matches('/');
        if s.is_empty() {
            return Ok("/".to_string());
        }
        let parts: Vec<&str> = s
            .split('/')
            .filter(|p| !p.is_empty() && *p != ".")
            .collect();
        let s = parts.join("/");
        if s.is_empty() {
            return Err(MegaError::Buck(BuckError::ValidationError(
                "Path cannot be empty or consist only of '.' segments".to_string(),
            )));
        }
        Ok(format!("/{}", s))
    }

    /// Validate a GitHub sync target path (`/third-party/...` or `/project/...` subdirectories).
    pub fn validate_github_sync_path(path: &str) -> Result<String, MegaError> {
        let normalized = Self::normalize_repo_path(path)?;
        if normalized == "/third-party" || normalized == "/project" {
            return Err(MegaError::Buck(BuckError::ValidationError(
                "GitHub sync path must be a subdirectory under /third-party or /project"
                    .to_string(),
            )));
        }
        if normalized.starts_with("/third-party/") || normalized.starts_with("/project/") {
            return Ok(normalized);
        }
        Err(MegaError::Buck(BuckError::ValidationError(
            "GitHub sync path must start with /third-party/ or /project/".to_string(),
        )))
    }

    /// Returns true when a receive-pack status report contains a failed ref update (`ng`).
    pub fn receive_pack_report_failed(report: &Bytes) -> bool {
        String::from_utf8_lossy(report).contains("ng refs/")
    }

    /// True when a CL represents a brand-new monorepo subdirectory (no prior main ref).
    pub fn is_new_directory_cl(from_hash: &str) -> bool {
        from_hash == ZERO_ID
    }

    /// Enumerate candidate repo roots from the deepest directory back to `/`.
    pub fn repo_root_candidates(path: &Path) -> Vec<String> {
        let mut current = PathBuf::from("/").join(path);
        let mut candidates = Vec::new();

        loop {
            candidates.push(Self::clean_path_str(&current.to_string_lossy()));
            if !current.pop() {
                break;
            }
        }

        candidates
    }

    pub fn subtree_ref_path(path: &Path) -> Result<String, MegaError> {
        Self::normalize_repo_path(&path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use common::errors::{BuckError, MegaError};

    use super::MonoServiceLogic;

    #[test]
    fn test_clean_path_str_edges() {
        assert_eq!(MonoServiceLogic::clean_path_str(""), "/");
        assert_eq!(MonoServiceLogic::clean_path_str("/"), "/");
        assert_eq!(MonoServiceLogic::clean_path_str("abc/"), "abc");
        assert_eq!(MonoServiceLogic::clean_path_str("abc///"), "abc");
    }

    #[test]
    fn test_normalize_repo_path() {
        // Normalization: add leading slash, strip trailing
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("project").unwrap(),
            "/project"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("/project").unwrap(),
            "/project"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("project/").unwrap(),
            "/project"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("/project/").unwrap(),
            "/project"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("  /project  ").unwrap(),
            "/project"
        );
        assert_eq!(MonoServiceLogic::normalize_repo_path("/").unwrap(), "/");

        // Empty / whitespace-only -> ValidationError
        assert!(MonoServiceLogic::normalize_repo_path("").is_err());
        assert!(MonoServiceLogic::normalize_repo_path("   ").is_err());
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path(""),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));

        // Path traversal and invalid chars -> ValidationError
        assert!(MonoServiceLogic::normalize_repo_path("project/../foo").is_err());
        assert!(MonoServiceLogic::normalize_repo_path("project\\foo").is_err());

        // Middle slashes and "." segments are collapsed
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("//project//foo//").unwrap(),
            "/project/foo"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("project/./foo").unwrap(),
            "/project/foo"
        );
        assert_eq!(
            MonoServiceLogic::normalize_repo_path("/project/./foo").unwrap(),
            "/project/foo"
        );

        // Dot-only paths are rejected (do not silently resolve to root)
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path("."),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path("./"),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path("./."),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));

        // Leading colon is rejected
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path(":/test"),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path(":"),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));

        // Windows drive letters are rejected
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path("C:"),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));
        assert!(matches!(
            MonoServiceLogic::normalize_repo_path("D:/project"),
            Err(MegaError::Buck(BuckError::ValidationError(_)))
        ));
    }

    #[test]
    fn test_repo_root_candidates_walk_from_leaf_to_root() {
        assert_eq!(
            MonoServiceLogic::repo_root_candidates(Path::new("/project/buck2_test/src")),
            vec![
                "/project/buck2_test/src".to_string(),
                "/project/buck2_test".to_string(),
                "/project".to_string(),
                "/".to_string(),
            ]
        );
    }

    #[test]
    fn test_repo_root_candidates_normalize_relative_paths() {
        assert_eq!(
            MonoServiceLogic::repo_root_candidates(Path::new("project/buck2_test/src")),
            vec![
                "/project/buck2_test/src".to_string(),
                "/project/buck2_test".to_string(),
                "/project".to_string(),
                "/".to_string(),
            ]
        );
    }

    #[test]
    fn test_subtree_ref_path_keeps_parent_directory_for_file_edits() {
        assert_eq!(
            MonoServiceLogic::subtree_ref_path(Path::new("/project/buck2_test/src")).unwrap(),
            "/project/buck2_test/src".to_string()
        );
    }

    #[test]
    fn test_subtree_ref_path_normalizes_relative_create_paths() {
        assert_eq!(
            MonoServiceLogic::subtree_ref_path(Path::new("project/buck2_test/src")).unwrap(),
            "/project/buck2_test/src".to_string()
        );
    }

    #[test]
    fn test_path_traversal_with_pop() {
        let mut full_path = PathBuf::from("/project/rust/mega");
        for _ in 0..3 {
            let cloned_path = full_path.clone();
            let name = cloned_path.file_name().unwrap().to_str().unwrap();
            full_path.pop();
            println!("name: {name}, path: {full_path:?}");
        }
    }
}
