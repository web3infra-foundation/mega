use clap::Subcommand;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use reqwest::StatusCode;
use ceres::lfs::lfs_structs::LockListQuery;
use crate::command::ask_basic_auth;
use crate::internal::head::Head;
use crate::internal::protocol::lfs_client::LFS_CLIENT;
use crate::utils::{lfs, path, util};
use crate::utils::path_ext::PathExt;

#[derive(Subcommand, Debug)]
pub enum LfsCmds {
    /// View or add LFS paths to Libra Attributes (root)
    Track {
        pattern: Option<Vec<String>>,
    },
    /// Remove LFS paths from Libra Attributes
    Untrack {
        path: Vec<String>,
    },
    /// Lists currently locked files from the Libra LFS server. (Current Branch)
    Locks {
        #[clap(long, short)]
        id: Option<String>,
        #[clap(long, short)]
        path: Option<String>,
        #[clap(long, short)]
        limit: Option<u64>,
    },
    /// Set a file as "locked" on the Libra LFS server
    Lock {
        /// String path name of the locked file. This should be relative to the root of the repository working directory
        path: String,
    }
}

pub async fn execute(cmd: LfsCmds) {
    // TODO: attributes file should be created in current dir, NOT root dir
    let attr_path = path::attributes().to_string_or_panic();
    match cmd {
        LfsCmds::Track { pattern } => { // TODO: deduplicate
            match pattern {
                Some(pattern) => {
                    let pattern = convert_patterns_to_workdir(pattern); //
                    add_lfs_patterns(&attr_path, pattern).unwrap();
                }
                None => {
                    let lfs_patterns = lfs::extract_lfs_patterns(&attr_path).unwrap();
                    if !lfs_patterns.is_empty() {
                        println!("Listing tracked patterns");
                        for p in lfs_patterns {
                            println!("    {} ({})", p, util::ATTRIBUTES); // '\t' seems to be 8 spaces, :(
                        }
                    }
                }
            }
        }
        LfsCmds::Untrack { path } => {
            let path = convert_patterns_to_workdir(path); //
            untrack_lfs_patterns(&attr_path, path).unwrap();
        }
        LfsCmds::Locks { id, path, limit } => {
            let refspec = current_refspec().await.unwrap();
            tracing::debug!("refspec: {}", refspec);
            let query = LockListQuery {
                id: id.unwrap_or_default(),
                path: path.unwrap_or_default(),
                limit: limit.map(|l| l.to_string()).unwrap_or_default(),
                cursor: "".to_string(),
                refspec,
            };
            let locks = LFS_CLIENT.await.get_locks(query).await.locks;
            if !locks.is_empty() {
                for lock in locks {
                    println!("{}\tID:{}", lock.path, lock.id);
                }
            }
        }
        LfsCmds::Lock { path } => {
            if !lfs::is_lfs_tracked(&path) {
                eprintln!("fatal: {} is not an LFS tracked file", path);
                return;
            }

            let refspec = current_refspec().await.unwrap();
            let mut auth = None;
            loop {
                let resp = LFS_CLIENT.await.lock(path.clone(), refspec.clone(), auth.clone()).await;
                let code = resp.status();
                if code.is_success() {
                    println!("Locked {}", path);
                    return;
                } else if code == StatusCode::FORBIDDEN {
                    eprintln!("Forbidden: You must have push access to create a lock");
                    auth = Some(ask_basic_auth());
                    continue;
                } else if code == StatusCode::CONFLICT {
                    eprintln!("Conflict: already created lock");
                } else if !code.is_success() {
                    eprintln!("fatal: LFS lock failed. Code: {}, Message: {}", code, resp.text().await.unwrap());
                }
                break;
            }
        }
    }
}

async fn current_refspec() -> Option<String> {
    match Head::current().await {
        Head::Branch(name) => Some(format!("refs/heads/{}", name)),
        Head::Detached(_) => {
            println!("fatal: HEAD is detached");
            None
        }
    }
}

/// temp
fn convert_patterns_to_workdir(patterns: Vec<String>) -> Vec<String> {
    patterns.into_iter().map(|p| {
        util::to_workdir_path(&p).to_string_or_panic()
    }).collect()
}

fn add_lfs_patterns(file_path: &str, patterns: Vec<String>) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(file_path)?;

    if file.metadata()?.len() > 0 {
        file.seek(SeekFrom::End(-1))?;

        let mut last_byte = [0; 1];
        file.read_exact(&mut last_byte)?;

        // ensure the last byte is '\n'
        if last_byte[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }

    let lfs_patterns = lfs::extract_lfs_patterns(file_path)?;
    for pattern in patterns {
        if lfs_patterns.contains(&pattern) {
            continue;
        }
        println!("Tracking \"{}\"", pattern);
        let pattern = format!("{} filter=lfs diff=lfs merge=lfs -text\n", pattern.replace(" ", r"\ "));
        file.write_all(pattern.as_bytes())?;
    }

    Ok(())
}

fn untrack_lfs_patterns(file_path: &str, patterns: Vec<String>) -> io::Result<()> {
    if !Path::new(file_path).exists() {
        return Ok(());
    }
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let mut matched_pattern = None;
        // delete the specified lfs patterns
        for pattern in &patterns {
            let pattern = pattern.replace(" ", r"\ ");
            if line.trim_start().starts_with(&pattern) && line.contains("filter=lfs") {
                matched_pattern = Some(pattern);
                break;
            }
        }
        match matched_pattern {
            Some(pattern) => println!("Untracking \"{}\"", pattern),
            None => lines.push(line),
        }
    }

    // clear the file
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    for line in lines {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}