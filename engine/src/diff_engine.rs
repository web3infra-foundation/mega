use std::collections::HashMap;
use std:: {
    io::{self},
    path::{Path, PathBuf},
    fmt::Write
};
use std::collections::HashSet;
use imara_diff::{
    Algorithm,
    Diff,
    UnifiedDiffConfig,
    BasicLineDiffPrinter
};
use imara_diff::InternedInput;
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
pub struct DiffEngine;

impl DiffEngine {
    /// Computes and writes unified diffs for changed files between two blob sets.
    ///
    /// This is the main entry point for the diff engine. It compares files between
    /// old and new blob collections, applies filtering, and writes the results in
    /// unified diff format.
    ///
    /// # Arguments
    ///
    /// * `old_blobs` - Vector of (path, hash) tuples representing the old file state
    /// * `new_blobs` - Vector of (path, hash) tuples representing the new file state
    /// * `algorithm` - Diff algorithm to use ("myers", "myersMinimal", or "histogram")
    /// * `filter` - List of paths to filter; empty means process all files
    /// * `w` - Writer to output the diff results
    /// * `read_content` - Function to read file content given a path and hash
    ///
    /// # Algorithm Options
    ///
    /// - `"myers"` - Standard Myers algorithm
    /// - `"myersMinimal"` - Myers algorithm optimized for minimal diffs
    /// - `"histogram"` - Histogram algorithm (default, generally fastest)
    pub async fn diff(
        old_blobs: Vec<(PathBuf, SHA1)>,
        new_blobs: Vec<(PathBuf, SHA1)>,
        algorithm: String,
        filter: Vec<PathBuf>,
        w: &mut dyn io::Write,
        read_content: &dyn Fn(&PathBuf, &SHA1) -> Vec<u8>,
    ){
        let old_blobs: HashMap<PathBuf, SHA1> = old_blobs.into_iter().collect();
        let new_blobs: HashMap<PathBuf, SHA1> = new_blobs.into_iter().collect();

        // union set
        let union_files: HashSet<PathBuf> = old_blobs.keys().chain(new_blobs.keys()).cloned().collect();
        tracing::debug!(
            "old_blobs: {:?}, new_blobs: {:?}, union_files: {:?}",
            old_blobs.len(),
            new_blobs.len(),
            union_files.len()
        );

        // filter files, cross old and new files, and pathspec
        for file in union_files {
            if Self::should_process(&file, &filter, &old_blobs, &new_blobs) {
                Self::write_diff_for_file(&file, &old_blobs, &new_blobs, algorithm.as_str(), w, &read_content);
            }
        }
    }

    pub async fn mono_diff<F>(
        old_blobs: Vec<(PathBuf, SHA1)>,
        new_blobs: Vec<(PathBuf, SHA1)>,
        algorithm: String,
        filter: Vec<PathBuf>,
        read_content: F,
    ) -> Vec<String> 
    where
        F: Fn(&PathBuf, &SHA1) -> Vec<u8>,
    {
        let old_blobs: HashMap<PathBuf, SHA1> = old_blobs.into_iter().collect();
        let new_blobs: HashMap<PathBuf, SHA1> = new_blobs.into_iter().collect();

        // union set
        let union_files: HashSet<PathBuf> = old_blobs.keys().chain(new_blobs.keys()).cloned().collect();
        tracing::debug!(
            "old_blobs: {:?}, new_blobs: {:?}, union_files: {:?}",
            old_blobs.len(),
            new_blobs.len(),
            union_files.len()
        );

        let mut diffs = Vec::new();

        for file in union_files {
            if Self::should_process(&file, &filter, &old_blobs, &new_blobs) {
                let diff = Self::diff_for_file_string(
                    &file,
                    &old_blobs,
                    &new_blobs,
                    algorithm.as_str(),
                    &read_content,
                );
                diffs.push(diff);
            }
        }

        diffs
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

                // call your diff engine; here I'll inline a placeholder
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

    fn write_diff_for_file(
        file: &PathBuf,
        old_blobs: &HashMap<PathBuf, SHA1>,
        new_blobs: &HashMap<PathBuf, SHA1>,
        algorithm: &str,
        w: &mut dyn io::Write,
        read_content: &dyn Fn(&PathBuf, &SHA1) -> Vec<u8>,
    ) {
        let new_hash = new_blobs.get(file);
        let old_hash = old_blobs.get(file);

        let old_content = old_hash.map_or_else(Vec::new, |h| read_content(file, h));
        let new_content = new_hash.map_or_else(Vec::new, |h| read_content(file, h));

        writeln!(
            w,
            "diff --git a/{} b/{}",
            file.display(),
            file.display()
        ).unwrap();

        if old_hash.is_none() {
            writeln!(w, "new file mode 100644").unwrap();
        } else if new_hash.is_none() {
            writeln!(w, "deleted file mode 100644").unwrap();
        }

        let old_index = old_hash.map_or("0000000".to_string(), |h| h.to_string()[0..8].to_string());
        let new_index = new_hash.map_or("0000000".to_string(), |h| h.to_string()[0..8].to_string());
        writeln!(w, "index {old_index}..{new_index}").unwrap();

        let old_type = infer::get(&old_content);
        let new_type = infer::get(&new_content);
        match (String::from_utf8(old_content), String::from_utf8(new_content)) {
            (Ok(old_text), Ok(new_text)) => {
                let (old_prefix, new_prefix) = if old_text.is_empty() {
                    (
                        "/dev/null".to_string(),
                        format!("b/{}", Self::file_display(file, new_hash, new_type)),
                    )
                } else if new_text.is_empty() {
                    (
                        format!("a/{}", Self::file_display(file, old_hash, old_type)),
                        "/dev/null".to_string(),
                    )
                } else {
                    (
                        format!("a/{}", Self::file_display(file, old_hash, old_type)),
                        format!("b/{}", Self::file_display(file, new_hash, new_type)),
                    )
                };
                writeln!(w, "--- {old_prefix}").unwrap();
                writeln!(w, "+++ {new_prefix}").unwrap();
                Self::imara_diff_result(&old_text, &new_text, algorithm, w);
            }
            _ => {
                // TODO: Handle non-UTF-8 data as binary for now; consider optimization in the future.
                writeln!(
                    w,
                    "Binary files a/{} and b/{} differ",
                    Self::file_display(file, old_hash, old_type),
                    Self::file_display(file, new_hash, new_type)
                ).unwrap();
            }
        }
    }

    // display file with type
    fn file_display(file: &Path, hash: Option<&SHA1>, file_type: Option<infer::Type>) -> String {
        let file_name = match hash {
            Some(_) => file.display().to_string(),
            None => "dev/null".to_string(),
        };

        if let Some(file_type) = file_type {
            // Check if the file type is displayable in browser, like image, audio, video, etc.
            if matches!(
            file_type.matcher_type(),
            infer::MatcherType::Audio | infer::MatcherType::Video | infer::MatcherType::Image
        ) {
                return format!("{} ({})", file_name, file_type.mime_type()).to_string();
            }
        }
        file_name
    }

    fn imara_diff_result(old: &str, new: &str, algorithm: &str, w: &mut dyn io::Write) {
        let input = InternedInput::new(old, new);

        let algo = match algorithm {
            "myers" => Algorithm::Myers,
            "myersMinimal" => Algorithm::MyersMinimal,
            // default is the histogram algo
            _ => Algorithm::Histogram,
        };
        tracing::debug!("libra [diff]: choose the algorithm: {:?}", algo);

        let mut diff = Diff::compute(algo, &input);

        // did the postprocess_lines
        diff.postprocess_lines(&input);

        let result = diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string();

        write!(w, "{result}").unwrap();
    }

}

mod tests {
    #[test]
    fn test_diff_algorithms_correctness_and_efficiency() {
        let old = r#"function foo() {
    if (condition) {
        doSomething();
        doSomethingElse();
        andAnotherThing();
    } else {
        alternative();
    }
}"#;

        let new = r#"function foo() {
    if (condition) {
        // Added comment
        doSomething();
        // Modified this line
        modifiedSomethingElse();
        andAnotherThing();
    } else {
        alternative();
    }

    // Added new block
    addedNewFunctionality();
}"#;
        let mut outputs = Vec::new();

        let algos = ["histogram", "myers", "myersMinimal"];

        // test the different algo benchmark
        for algo in algos {
            let mut buf = Vec::new();
            let start = tokio::time::Instant::now();
            crate::DiffEngine::imara_diff_result(old, new, algo, &mut buf);
            let elapse = start.elapsed();
            let ouput = String::from_utf8(buf).expect("Invalid UTF-8 in diff ouput");

            println!("libra diff algorithm: {algo:?} Spend Time: {elapse:?}");
            assert!(
                !ouput.is_empty(),
                "libra diff algorithm: {algo} produce a empty output"
            );
            assert!(
                ouput.contains("@@"),
                "libra diff algorithm: {algo}, ouput missing diff markers"
            );

            outputs.push((algo, ouput));
        }

        // check the line counter difference
        for (algo, output) in outputs {
            let plus_line = output.lines().filter(|line| line.starts_with("+")).count();
            let minus_line = output.lines().filter(|line| line.starts_with("-")).count();
            assert_eq!(
                plus_line, 6,
                "libra diff algorithm {algo}, expect plus_line: 6, got {plus_line} "
            );
            assert_eq!(
                minus_line, 1,
                "libra diff algorithm {algo}, expect minus_line: 1, got {minus_line} "
            );
        }
    }
}

