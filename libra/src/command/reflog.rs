use crate::command::{load_object, HEAD};
use crate::internal::config;
use crate::internal::db::get_db_conn_instance;
use crate::internal::model::reflog::Model;
use crate::internal::reflog::{Reflog, ReflogError};
use clap::{Parser, Subcommand};
use colored::Colorize;
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use sea_orm::sqlx::types::chrono;
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::process::{Command, Stdio};
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct ReflogArgs {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug, Clone)]
enum Subcommands {
    /// show reflog records.
    Show {
        #[clap(default_value = "HEAD")]
        ref_name: String,
        #[arg(long = "pretty")]
        #[clap(default_value_t = FormatterKind::default())]
        pretty: FormatterKind,
    },
    /// clear the reflog record of the specified branch.
    Delete {
        #[clap(required = true, num_args = 1..)]
        selectors: Vec<String>,
    },
    /// check whether a reference has a reflog record, usually using by automatic scripts.
    Exists {
        #[clap(required = true)]
        ref_name: String,
    },
}

pub async fn execute(args: ReflogArgs) {
    match args.command {
        Subcommands::Show { ref_name, pretty } => handle_show(&ref_name, pretty).await,
        Subcommands::Delete { selectors } => handle_delete(&selectors).await,
        Subcommands::Exists { ref_name } => handle_exists(&ref_name).await,
    }
}

async fn handle_show(ref_name: &str, pretty: FormatterKind) {
    let db = get_db_conn_instance().await;

    let ref_name = parse_ref_name(ref_name).await;
    let logs = match Reflog::find_all(db, &ref_name).await {
        Ok(logs) => logs,
        Err(e) => {
            eprintln!("fatal: failed to get reflog entries: {e}");
            return;
        }
    };

    let formatter = ReflogFormatter {
        logs: &logs,
        kind: pretty,
    };

    #[cfg(unix)]
    let mut less = Command::new("less") // create a pipe to less
        .arg("-R") // raw control characters
        .arg("-F")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("failed to execute process");

    #[cfg(unix)]
    if let Some(ref mut stdin) = less.stdin {
        writeln!(stdin, "{formatter}").expect("fatal: failed to write to stdin");
    } else {
        eprintln!("Failed to capture stdin");
    }

    #[cfg(unix)]
    let _ = less.wait().expect("failed to wait on child");

    #[cfg(not(unix))]
    println!("{formatter}")
}

// `partial_ref_name` is the branch name entered by the user.
async fn parse_ref_name(partial_ref_name: &str) -> String {
    if partial_ref_name == HEAD {
        return HEAD.to_string();
    }
    if !partial_ref_name.contains("/") {
        return format!("refs/heads/{partial_ref_name}");
    }
    let (ref_name, _) = partial_ref_name.split_once("/").unwrap();
    if config::Config::get("remote", Some(ref_name), "url")
        .await
        .is_some()
    {
        return format!("refs/remotes/{partial_ref_name}");
    }
    format!("refs/heads/{partial_ref_name}")
}

async fn handle_exists(ref_name: &str) {
    let db = get_db_conn_instance().await;
    let log = Reflog::find_one(db, ref_name)
        .await
        .expect("fatal: failed to get reflog entry");
    match log {
        Some(_) => {}
        None => std::process::exit(1),
    }
}

async fn handle_delete(selectors: &[String]) {
    let mut groups = HashMap::new();
    for selector in selectors {
        if let Some(parsed) = parse_reflog_selector(selector) {
            groups
                .entry(parsed.0.to_string())
                .or_insert_with(Vec::new)
                .push(parsed);
            continue;
        }
        eprintln!("fatal: invalid reflog entry format: {selector}");
        return;
    }

    let groups = groups
        .into_values()
        .map(|mut group| {
            group.sort_by(|a, b| b.1.cmp(&a.1));
            group
        })
        .collect::<Vec<_>>();
    for group in groups {
        delete_single_group(&group).await;
    }
}

async fn delete_single_group(group: &[(&str, usize)]) {
    let db = get_db_conn_instance().await;
    // clone this to move it into async block to make compiler happy :(
    let group = group
        .iter()
        .map(|(s, i)| ((*s).to_string(), *i))
        .collect::<Vec<(String, usize)>>();

    db.transaction(|txn| {
        Box::pin(async move {
            let ref_name = &group[0].0;
            let logs = Reflog::find_all(txn, ref_name).await?;

            for (_, index) in &group {
                if let Some(entry) = logs.get(*index) {
                    let id = entry.id;
                    txn.execute(Statement::from_sql_and_values(
                        DbBackend::Sqlite,
                        "DELETE FROM reflog WHERE id = ?;",
                        [id.into()],
                    ))
                    .await?;
                    continue;
                }
                eprintln!("fatal: reflog entry `{ref_name}@{{{index}}}` not found")
            }

            Ok::<_, ReflogError>(())
        })
    })
    .await
    .expect("fatal: failed to delete reflog entries")
}

fn parse_reflog_selector(selector: &str) -> Option<(&str, usize)> {
    if let (Some(at_brace), Some(end_brace)) = (selector.find("@{"), selector.find('}')) {
        if at_brace < end_brace {
            let ref_name = &selector[..at_brace];
            let index_str = &selector[at_brace + 2..end_brace];

            if let Ok(index) = index_str.parse::<usize>() {
                return Some((ref_name, index));
            }
        }
    }
    None
}

#[derive(Debug, Copy, Clone)]
enum FormatterKind {
    Oneline,
    Short,
    Medium,
    Full,
}

impl Display for FormatterKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Oneline => f.write_str("oneline"),
            Self::Short => f.write_str("short"),
            Self::Medium => f.write_str("medium"),
            Self::Full => f.write_str("full"),
        }
    }
}

impl Default for FormatterKind {
    fn default() -> Self {
        Self::Oneline
    }
}

impl From<String> for FormatterKind {
    fn from(value: String) -> Self {
        match value.as_str() {
            "oneline" => FormatterKind::Oneline,
            "short" => FormatterKind::Short,
            "medium" => FormatterKind::Medium,
            "full" => FormatterKind::Full,
            _ => FormatterKind::Oneline,
        }
    }
}

struct ReflogFormatter<'a> {
    logs: &'a Vec<Model>,
    kind: FormatterKind,
}

impl<'a> Display for ReflogFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let all = self.logs
            .iter()
            .enumerate()
            .map(|(idx, log)| {
                let head = format!("HEAD@{{{idx}}}");
                let new_oid = &log.new_oid[..7];

                let commit = find_commit(&log.new_oid);
                let full_msg = format!("{}: {}", log.action, log.message);

                let author = format!("{} <{}>", commit.author.name, commit.author.email);
                let committer = format!("{} <{}>", log.committer_name, log.committer_email);
                let commit_msg = &commit.message.trim();
                let datetime = format_datetime(log.timestamp);

                match self.kind {
                    FormatterKind::Oneline => format!(
                        "{} {head}: {full_msg}",
                        new_oid.to_string().bright_magenta(),
                    ),
                    FormatterKind::Short => format!(
                        "{}\nReflog: {head} ({author})\nReflog message: {full_msg}\nAuthor: {author}\n\n  {commit_msg}\n",
                        format!("commit {new_oid}").bright_magenta(),
                    ),
                    FormatterKind::Medium => format!(
                        "{}\nReflog: {head} ({author})\nReflog message: {full_msg}\nAuthor: {author}\nDate:   {datetime}\n\n  {commit_msg}\n",
                        format!("commit {new_oid}").bright_magenta(),
                    ),
                    FormatterKind::Full => format!(
                        "{}\nReflog: {head} ({author})\nReflog message: {full_msg}\nAuthor: {author}\nCommit: {committer}\n\n  {commit_msg}\n",
                        format!("commit {new_oid}").bright_magenta(),
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(f, "{all}")
    }
}

fn find_commit(commit_hash: &str) -> Commit {
    let hash = SHA1::from_str(commit_hash).unwrap();
    load_object::<Commit>(&hash).unwrap()
}

fn format_datetime(timestamp: i64) -> String {
    let naive = chrono::DateTime::from_timestamp(timestamp, 0).unwrap();
    let local = naive.with_timezone(&chrono::Local);

    let git_format = "%a %b %d %H:%M:%S %Y %z";
    local.format(git_format).to_string()
}
