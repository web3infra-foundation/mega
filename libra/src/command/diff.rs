use std::{fmt, io};

use clap::Parser;
use mercury::internal::{index::Index, object::tree::Tree};
use similar;

use crate::{
    command::{get_target_commit, status, HEAD},
    internal::head::Head,
    utils::{client_storage::ClientStorage, path, util},
};
#[derive(Parser, Debug)]
pub struct DiffArgs {
    #[clap(long, help = "Old commit, defaults is staged or HEAD")]
    pub old: Option<String>,

    #[clap(long, help = "New commit, default is working directory")]
    #[clap(requires = "old", group = "op_new")]
    pub new: Option<String>,

    #[clap(long, help = "use stage as new commit")]
    #[clap(group = "op_new")]
    pub staged: bool,

    #[clap(help = "Files to compare")]
    pathspec: Vec<String>,

    #[clap(long)]
    pub output: Option<String>,
}

pub async fn execute(args: DiffArgs) {
    let mut w = match args.output {
        Some(ref path) => {
            let file = std::fs::File::create(path)
                .map_err(|e| {
                    eprintln!(
                        "fatal: could not open to file '{}' for writing: {}",
                        path, e
                    );
                })
                .unwrap();
            Box::new(file) as Box<dyn io::Write>
        }
        None => Box::new(io::stdout()) as Box<dyn io::Write>,
    };

    diff_result("Hello World\nHello Life", "Hallo Welt\nHello Work", &mut w);
    unimplemented!();
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn diff_result(old: &str, new: &str, w: &mut dyn io::Write) {
    let diff = similar::TextDiff::from_lines(old, new);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "-",
                    similar::ChangeTag::Insert => "+",
                    similar::ChangeTag::Equal => " ",
                };
                write!(
                    w,
                    "{}{} |{}",
                    Line(change.old_index()),
                    Line(change.new_index()),
                    sign
                )
                .unwrap();
                write!(w, "{}", change.value()).unwrap();
                if change.missing_newline() {
                    writeln!(w).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_args() {
        {
            let args = DiffArgs::try_parse_from(["diff", "--old", "old", "--new", "new", "paths"]);
            assert!(args.is_ok());
            let args = args.unwrap();
            // println!("{:?}", args);
            assert_eq!(args.old, Some("old".to_string()));
            assert_eq!(args.new, Some("new".to_string()));
            assert_eq!(args.pathspec, vec!["paths".to_string()]);
        }
        {
            // --staged didn't require --old
            let args =
                DiffArgs::try_parse_from(["diff", "--staged", "pathspec", "--output", "output"]);
            let args = args.unwrap();
            assert_eq!(args.old, None);
            assert!(args.staged);
        }
        {
            // --staged conflicts with --new
            let args = DiffArgs::try_parse_from([
                "diff", "--old", "old", "--new", "new", "--staged", "paths",
            ]);
            assert!(args.is_err());
            assert!(args.err().unwrap().kind() == clap::error::ErrorKind::ArgumentConflict);
        }
        {
            // --new requires --old
            let args = DiffArgs::try_parse_from([
                "diff", "--new", "new", "pathspec", "--output", "output",
            ]);
            assert!(args.is_err());
            assert!(args.err().unwrap().kind() == clap::error::ErrorKind::MissingRequiredArgument);
        }
    }

    #[test]
    fn test_diff_result() {
        let old = "Hello World\nThis is the second line.\nThis is the third.";
        let new = "Hallo Welt\nThis is the second line.\nThis is life.\nMoar and more";
        let mut buf = Vec::new();
        diff_result(old, new, &mut buf);
        let result = String::from_utf8(buf).unwrap();
        println!("{}", result);
    }
}
