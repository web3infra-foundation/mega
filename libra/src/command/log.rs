use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::command::load_object;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use clap::Parser;
use colored::Colorize;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::process::{Command, Stdio};

use mercury::hash::SHA1;
use mercury::internal::object::{blob::Blob, commit::Commit, tree::Tree};
use neptune::Diff;
use std::collections::VecDeque;
use std::str::FromStr;

use crate::utils::object_ext::TreeExt;
use crate::utils::util;
use common::utils::parse_commit_msg;
#[derive(Parser, Debug)]
pub struct LogArgs {
    /// Limit the number of output
    #[clap(short, long)]
    pub number: Option<usize>,
    /// Shorthand for --pretty=oneline --abbrev-commit
    #[clap(long)]
    pub oneline: bool,
    /// Show diffs for each commit (like git -p)
    #[clap(short = 'p', long = "patch")]
    pub patch: bool,
    /// Print out ref names of any commits that are shown
    #[clap(
        long,
        default_missing_value = "short",
        require_equals = true,
        num_args = 0..=1,
    )]
    pub decorate: Option<String>,
    /// Do not print out ref names of any commits that are shown
    #[clap(long)]
    pub no_decorate: bool,

    /// Files to limit diff output (only used with -p)
    #[clap(requires = "patch", value_name = "PATHS")]
    pathspec: Vec<String>,
}

#[derive(PartialEq)]
enum DecorateOptions {
    No,
    Short,
    Full,
}

fn str_to_decorate_option(s: &str) -> Result<DecorateOptions, String> {
    match s {
        "no" => Ok(DecorateOptions::No),
        "short" => Ok(DecorateOptions::Short),
        "full" => Ok(DecorateOptions::Full),
        "auto" => {
            if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                Ok(DecorateOptions::Short)
            } else {
                Ok(DecorateOptions::No)
            }
        }
        _ => Err(s.to_owned()),
    }
}

async fn determine_decorate_option(args: &LogArgs) -> Result<DecorateOptions, String> {
    let arg_deco = args
        .decorate
        .as_ref()
        .map(|s| str_to_decorate_option(s))
        .transpose()?;

    if arg_deco.is_some() && args.no_decorate {
        let mut args_os = std::env::args_os().peekable();
        while let Some(arg) = args_os.next() {
            if arg == "--no-decorate" {
                return Ok(arg_deco.unwrap());
            } else if arg.to_str().unwrap_or_default().starts_with("--decorate") {
                return Ok(DecorateOptions::No);
            };
        }
    } else if arg_deco.is_some() {
        return Ok(arg_deco.unwrap());
    } else if args.no_decorate {
        return Ok(DecorateOptions::No);
    };

    if let Some(config_deco) = Config::get("log", None, "decorate")
        .await
        .map(|s| str_to_decorate_option(&s).ok())
        .flatten()
    {
        Ok(config_deco)
    } else {
        str_to_decorate_option("auto")
    }
}

///  Get all reachable commits from the given commit hash
///  **didn't consider the order of the commits**
pub async fn get_reachable_commits(commit_hash: String) -> Vec<Commit> {
    let mut queue = VecDeque::new();
    let mut commit_set: HashSet<String> = HashSet::new(); // to avoid duplicate commits because of circular reference
    let mut reachable_commits: Vec<Commit> = Vec::new();
    queue.push_back(commit_hash);

    while !queue.is_empty() {
        let commit_id = queue.pop_front().unwrap();
        let commit_id_hash = SHA1::from_str(&commit_id).unwrap();
        let commit = load_object::<Commit>(&commit_id_hash)
            .expect("fatal: storage broken, object not found");
        if commit_set.contains(&commit_id) {
            continue;
        }
        commit_set.insert(commit_id);

        let parent_commit_ids = commit.parent_commit_ids.clone();
        for parent_commit_id in parent_commit_ids {
            queue.push_back(parent_commit_id.to_string());
        }
        reachable_commits.push(commit);
    }
    reachable_commits
}

// Ordered as they should appear in log
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
enum ReferenceKind {
    Tag,    // decorate color = yellow
    Remote, // red
    Local,  // green
}

#[derive(PartialEq, Eq, Clone)]
struct Reference {
    name: String,
    kind: ReferenceKind,
}

impl PartialOrd for Reference {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.kind.cmp(&other.kind))
    }
}

impl Ord for Reference {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.kind.cmp(&other.kind)
    }
}

pub async fn execute(args: LogArgs) {
    let decorate_option = determine_decorate_option(&args)
        .await
        .expect("fatal: invalid --decorate option");

    #[cfg(unix)]
    let mut process = Command::new("less") // create a pipe to less
        .arg("-R") // raw control characters
        .arg("-F")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("failed to execute process");

    let head = Head::current().await;
    // check if the current branch has any commits
    let branch_name = if let Head::Branch(n) = head.to_owned() {
        Some(n)
    } else {
        None
    };
    if let Some(n) = &branch_name {
        let branch = Branch::find_branch(n, None).await;
        if branch.is_none() {
            panic!("fatal: your current branch '{n}' does not have any commits yet ");
        };
    };

    let commit_hash = Head::current_commit().await.unwrap().to_string();

    let mut reachable_commits = get_reachable_commits(commit_hash.clone()).await;
    // default sort with signature time
    reachable_commits.sort_by(|a, b| b.committer.timestamp.cmp(&a.committer.timestamp));

    let ref_commits = create_reference_commit_map().await;

    let max_output_number = min(args.number.unwrap_or(usize::MAX), reachable_commits.len());
    let mut output_number = 0;
    for commit in reachable_commits {
        if output_number >= max_output_number {
            break;
        }
        output_number += 1;

        let ref_msg = if decorate_option != DecorateOptions::No {
            let mut ref_msgs: Vec<String> = vec![];
            if output_number == 1 {
                ref_msgs.push(if let Some(b_name) = &branch_name {
                    format!(
                        "{} -> {}{}",
                        "HEAD".cyan(),
                        (if decorate_option == DecorateOptions::Full {
                            "refs/heads/"
                        } else {
                            ""
                        })
                        .green(),
                        b_name.green()
                    )
                } else {
                    "HEAD".cyan().to_string()
                });
            };

            let mut refs = ref_commits.get(&commit.id).cloned().unwrap_or_default();
            refs.sort();

            ref_msgs.append(
                &mut refs
                    .iter()
                    .filter_map(|r| {
                        if r.kind == ReferenceKind::Local && Some(r.name.to_owned()) == branch_name
                        {
                            None
                        } else {
                            Some(match r.kind {
                                ReferenceKind::Tag => format!(
                                    "tag: {}{}",
                                    if decorate_option == DecorateOptions::Full {
                                        "refs/tags/"
                                    } else {
                                        ""
                                    },
                                    r.name
                                )
                                .yellow()
                                .to_string(),
                                ReferenceKind::Remote => format!(
                                    "{}{}",
                                    if decorate_option == DecorateOptions::Full {
                                        "refs/remotes/"
                                    } else {
                                        ""
                                    },
                                    r.name
                                )
                                .red()
                                .to_string(),
                                ReferenceKind::Local => format!(
                                    "{}{}",
                                    if decorate_option == DecorateOptions::Full {
                                        "refs/heads/"
                                    } else {
                                        ""
                                    },
                                    r.name
                                )
                                .green()
                                .to_string(),
                            })
                        }
                    })
                    .collect(),
            );
            ref_msgs.join(", ")
        } else {
            String::new()
        };

        // prepare pathspecs for diff if needed
        let paths: Vec<PathBuf> = args.pathspec.iter().map(util::to_workdir_path).collect();

        let message = if args.oneline {
            // Oneline format: <short_hash> <refs> <commit_message_first_line>
            let short_hash = &commit.id.to_string()[..7];
            let (msg, _) = parse_commit_msg(&commit.message);
            if !ref_msg.is_empty() {
                format!("{} ({}) {}", short_hash.yellow().bold(), ref_msg, msg)
            } else {
                format!("{} {}", short_hash.yellow(), msg)
            }
        } else {
            // Default detailed format
            let mut message = if !ref_msg.is_empty() {
                format!(
                    "{} {} ({})",
                    "commit".yellow(),
                    commit.id.to_string().yellow().bold(),
                    ref_msg
                )
            } else {
                format!("{} {}", "commit".yellow(), commit.id.to_string().yellow())
            };

            message.push_str(&format!("\nAuthor: {}", commit.author));
            let (msg, _) = parse_commit_msg(&commit.message);
            message.push_str(&format!("\n{msg}\n"));
            // If patch requested, compute diff between this commit and its first parent
            if args.patch {
                let patch_output = generate_diff(&commit, paths.clone()).await;
                message.push_str(&patch_output);
            }

            message
        };

        #[cfg(unix)]
        {
            if let Some(ref mut stdin) = process.stdin {
                writeln!(stdin, "{message}").unwrap();
            } else {
                eprintln!("Failed to capture stdin");
            }
        }
        #[cfg(not(unix))]
        {
            println!("{message}");
        }
    }
    #[cfg(unix)]
    {
        let _ = process.wait().expect("failed to wait on child");
    }
}

async fn create_reference_commit_map() -> HashMap<SHA1, Vec<Reference>> {
    let mut commit_to_refs: HashMap<SHA1, Vec<Reference>> = HashMap::new();

    let all_branches = Branch::list_branches(None).await;
    for branch in all_branches {
        commit_to_refs
            .entry(branch.commit)
            .or_default()
            .push(match &branch.remote {
                Some(remote) => Reference {
                    name: format!("{}/{}", remote, branch.name),
                    kind: ReferenceKind::Remote,
                },
                None => Reference {
                    name: branch.name,
                    kind: ReferenceKind::Local,
                },
            });
    }

    let all_tags = crate::internal::tag::list().await.expect("fatal: ");
    for tag in all_tags {
        let commit_id = match tag.object {
            crate::internal::tag::TagObject::Commit(c) => c.id,
            crate::internal::tag::TagObject::Tag(t) => t.object_hash,
            _ => continue,
        };
        commit_to_refs
            .entry(commit_id)
            .or_default()
            .push(Reference {
                name: tag.name,
                kind: ReferenceKind::Tag,
            });
    }

    commit_to_refs
}

/// Generate unified diff between commit and its first parent (or empty tree)
async fn generate_diff(commit: &Commit, paths: Vec<PathBuf>) -> String {
    // prepare old and new blobs
    // new_blobs from commit tree
    let tree = load_object::<Tree>(&commit.tree_id).unwrap();
    let new_blobs: Vec<(PathBuf, SHA1)> = tree.get_plain_items();

    // old_blobs from first parent if exists
    let old_blobs: Vec<(PathBuf, SHA1)> = if !commit.parent_commit_ids.is_empty() {
        let parent = &commit.parent_commit_ids[0];
        let parent_hash = SHA1::from_str(&parent.to_string()).unwrap();
        let parent_commit = load_object::<Commit>(&parent_hash).unwrap();
        let parent_tree = load_object::<Tree>(&parent_commit.tree_id).unwrap();
        parent_tree.get_plain_items()
    } else {
        Vec::new()
    };

    let read_content = |file: &PathBuf, hash: &SHA1| match load_object::<Blob>(hash) {
        Ok(blob) => blob.data,
        Err(_) => {
            let file = util::to_workdir_path(file);
            std::fs::read(&file).unwrap()
        }
    };

    let diffs = Diff::diff(
        old_blobs,
        new_blobs,
        String::from("histogram"),
        paths.into_iter().collect(),
        read_content,
    )
    .await;
    let mut out = String::new();
    for d in diffs {
        out.push_str(&d.data);
    }
    out
}

#[cfg(test)]
mod tests {}
