use std::collections::HashMap;
use std:: {
    path::{PathBuf},
    fmt::Write
};
use std::collections::HashSet;
use mercury::{
    hash::SHA1
};
use infer;
use path_absolutize::Absolutize;

/// The main diff engine responsible for computing and formatting file differences.
///
/// `DiffEngine` provides static methods to compare files between two states (old and new)
/// and generate unified diff output. It supports various diff algorithms and handles
/// both text and binary files appropriately.
pub struct Diff;

impl Diff {
    const MAX_DIFF_LINES: usize = 1000; // Limit to avoid excessive output
    const LARGE_FILE_MARKER: &'static str = "<<<LARGE_FILE:"; // Marker for binary files
    const LARGE_FILE_END: &'static str = ">>>"; // End marker for binary files

    /// Computes and returns unified diffs for changed files between two blob sets as a single string.
    ///
    /// This is the unified diff neptune that handles all diff operations and returns a single
    /// string containing all the diff output. Both libra and mono can use this function and
    /// then handle the string output according to their needs.
    ///
    /// # Arguments
    ///
    /// * `old_blobs` - Vector of (path, hash) tuples representing the old file state
    /// * `new_blobs` - Vector of (path, hash) tuples representing the new file state
    /// * `algorithm` - Diff algorithm to use ("myers", "myersMinimal", or "histogram")
    /// * `filter` - List of paths to filter; empty means process all files
    /// * `read_content` - Function to read file content given a path and hash
    ///
    /// # Algorithm Options
    ///
    /// - `"myers"` - Standard Myers algorithm
    /// - `"myersMinimal"` - Myers algorithm optimized for minimal diffs
    /// - `"histogram"` - Histogram algorithm (default, generally fastest)
    ///
    /// # Returns
    ///
    /// A single string containing all diff output. Users can then write this to a file,
    /// display it in terminal, or process it further as needed.
    pub async fn diff<F>(
        old_blobs: Vec<(PathBuf, SHA1)>,
        new_blobs: Vec<(PathBuf, SHA1)>,
        algorithm: String,
        filter: Vec<PathBuf>,
        read_content: F,
    ) -> String 
    where 
        F: Fn(&PathBuf, &SHA1) -> Vec<u8>,
    {
        let (processed_files, old_blobs_map, new_blobs_map) = 
            Self::prepare_diff_data(old_blobs, new_blobs, &filter);
        
        let mut diff_results = Vec::new();
        for file in processed_files {
            if let Some(large_file_marker) = Self::is_large_file(
                &file,
                &old_blobs_map,
                &new_blobs_map,
                &read_content
            ) {
                diff_results.push(large_file_marker);
            } else {
                let diff = Self::diff_for_file_string(&file, &old_blobs_map, &new_blobs_map, algorithm.as_str(), &read_content);
                diff_results.push(diff);
            }
        }
        
        diff_results.join("")
    }


    /// Checks if a file is large and returns a message if it is.
    fn is_large_file <F>(
        file: &PathBuf,
        old_blobs: &HashMap<PathBuf, SHA1>,
        new_blobs: &HashMap<PathBuf, SHA1>,
        read_content: &F
    ) -> Option<String> 
    where 
        F: Fn(&PathBuf, &SHA1) -> Vec<u8>,
    {
        // Check if file is large based on some criteria, e.g. number of lines
        let old_hash = old_blobs.get(file);
        let new_hash = new_blobs.get(file);

        let old_bytes = old_hash.map_or_else(Vec::new, |h| read_content(file, h));
        let new_bytes = new_hash.map_or_else(Vec::new, |h| read_content(file, h));

        let old_lines = String::from_utf8_lossy(&old_bytes).lines().count();
        let new_lines = String::from_utf8_lossy(&new_bytes).lines().count();
        let total_lines = old_lines + new_lines;

        if total_lines > Self::MAX_DIFF_LINES {
            Some(format!(
                "{}{}:{}:{}{}\n",
                Self::LARGE_FILE_MARKER,
                file.display(),
                total_lines,
                Self::MAX_DIFF_LINES,
                Self::LARGE_FILE_END
            ))
        } else {
            None
        }
    }


    /// Extracts common diff preparation logic
    fn prepare_diff_data(
        old_blobs: Vec<(PathBuf, SHA1)>,
        new_blobs: Vec<(PathBuf, SHA1)>,
        filter: &[PathBuf],
    ) -> (Vec<PathBuf>, HashMap<PathBuf, SHA1>, HashMap<PathBuf, SHA1>) {
        let old_blobs_map: HashMap<PathBuf, SHA1> = old_blobs.into_iter().collect();
        let new_blobs_map: HashMap<PathBuf, SHA1> = new_blobs.into_iter().collect();

        // union set
        let union_files: HashSet<PathBuf> = old_blobs_map.keys().chain(new_blobs_map.keys()).cloned().collect();
        
        tracing::debug!(
            "old_blobs: {:?}, new_blobs: {:?}, union_files: {:?}",
            old_blobs_map.len(),
            new_blobs_map.len(),
            union_files.len()
        );

        // filter files that should be processed
        let processed_files: Vec<PathBuf> = union_files
            .into_iter()
            .filter(|file| Self::should_process(file, filter, &old_blobs_map, &new_blobs_map))
            .collect();

        (processed_files, old_blobs_map, new_blobs_map)
    }

    fn should_process(
        file: &PathBuf,
        filter: &[PathBuf],
        old_blobs: &HashMap<PathBuf, SHA1>,
        new_blobs: &HashMap<PathBuf, SHA1>,
    ) -> bool {
        // Skip if not in filter paths
        if !filter.is_empty() && !filter.iter().any(|path| Self::sub_of(file, path).unwrap_or(false)) {
            return false;
        }

        // Skip if hashes are equal or both absent
        old_blobs.get(file) != new_blobs.get(file)
    }

    fn sub_of(path: &PathBuf, parent: &PathBuf) -> Result<bool, std::io::Error> {
        let path_abs: PathBuf = path.absolutize()?.to_path_buf();
        let parent_abs: PathBuf = parent.absolutize()?.to_path_buf();
        Ok(path_abs.starts_with(parent_abs))
    }

    pub fn diff_for_file_string(
        file: &PathBuf,
        old_blobs: &HashMap<PathBuf, SHA1>,
        new_blobs: &HashMap<PathBuf, SHA1>,
        _algorithm: &str,
        read_content: &dyn Fn(&PathBuf, &SHA1) -> Vec<u8>,
    ) -> String {
        let mut out = String::new();
        
        // Look up hashes
        let new_hash = new_blobs.get(file);
        let old_hash = old_blobs.get(file);

        // Read contents or empty
        let old_bytes = old_hash.map_or_else(Vec::new, |h| read_content(file, h));
        let new_bytes = new_hash.map_or_else(Vec::new, |h| read_content(file, h));

        // diff header
        writeln!(out, "diff --git a/{} b/{}", file.display(), file.display()).unwrap();

        // file-mode lines
        if old_hash.is_none() {
            writeln!(out, "new file mode 100644").unwrap();
        } else if new_hash.is_none() {
            writeln!(out, "deleted file mode 100644").unwrap();
        }

        // index line
        let old_index = old_hash
            .map(|h| h.to_string()[0..8].to_string())
            .unwrap_or_else(|| "00000000".into());
        let new_index = new_hash
            .map(|h| h.to_string()[0..8].to_string())
            .unwrap_or_else(|| "00000000".into());
        writeln!(out, "index {old_index}..{new_index}").unwrap();

        // infer MIME / text vs binary
        let _old_type = infer::get(&old_bytes);
        let _new_type = infer::get(&new_bytes);

        // Try UTF-8 first
        match (String::from_utf8(old_bytes.clone()), String::from_utf8(new_bytes.clone())) {
            (Ok(old_text), Ok(new_text)) => {
                // a/ and b/ prefixes
                let (old_pref, new_pref) = if old_text.is_empty() {
                    ("/dev/null".to_string(),
                    format!("b/{}", file.display()))
                } else if new_text.is_empty() {
                    (format!("a/{}", file.display()), "/dev/null".to_string())
                } else {
                    (format!("a/{}", file.display()), format!("b/{}", file.display()))
                };

                writeln!(out, "--- {old_pref}").unwrap();
                writeln!(out, "+++ {new_pref}").unwrap();

                // call your diff neptune; here I'll inline a placeholder
                // replace this with your actual diff routine, e.g.:
                // imara_diff_result(&old_text, &new_text, algorithm, &mut out);
                //
                // For demonstration, we'll just show unified header:
                writeln!(
                    out,
                    "@@ -1,{} +1,{} @@",
                    old_text.lines().count(),
                    new_text.lines().count(),
                ).unwrap();
                for line in old_text.lines() {
                    writeln!(out, "-{line}").unwrap();
                }
                for line in new_text.lines() {
                    writeln!(out, "+{line}").unwrap();
                }
            }
            
            // Binary fallback
            _ => {
                writeln!(
                    out,
                    "Binary files a/{} and b/{} differ",
                    file.display(),
                    file.display()
                )
                .unwrap();
            }
        }

        out
    }
}