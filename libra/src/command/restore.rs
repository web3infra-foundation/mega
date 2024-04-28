use std::str::FromStr;
use clap::Parser;
use venus::hash::SHA1;
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::util;

#[derive(Parser, Debug)]
pub struct RestoreArgs {
    /// files or dir to restore
    #[clap(required = true)]
    pathspec: Vec<String>,
    /// source
    #[clap(long, short)]
    source: Option<String>,
    /// worktree
    #[clap(long, short = 'W')]
    worktree: bool,
    /// staged
    #[clap(long, short = 'S')]
    staged: bool,
}

pub async fn execute(args: RestoreArgs) {
    if !util::check_repo_exist() {
        return;
    }
    let staged = args.staged;
    let mut worktree = args.worktree;
    // If neither option is specified, by default the `working tree` is restored.
    // Specifying `--staged` will only restore the `index`. Specifying both restores both.
    if !staged {
        worktree = true;
    }

    let target_commit: Option<SHA1> = match args.source {
        None => {
            // If `--source` not specified, the contents are restored from `HEAD` if `--staged` is given,
            // otherwise from the [index].
            if staged {
                Head::current_commit().await // `HEAD`
            } else {
                None // Index
            }
        }
        Some(src) => {
            if src == "HEAD" {
                // Default Source
                Head::current_commit().await
            } else if Branch::exists(&src).await {
                // Branch Name, e.g. master
                Branch::current_commit(&src).await
            } else {
                // [Commit Hash, e.g. a1b2c3d4] || [Wrong Branch Name]
                let storage = util::objects_storage();
                let objs = storage.search(&src);
                if objs.len() != 1 { // TODO 判断objs[0]是否是commit!
                    None // Wrong Commit Hash
                } else {
                    Some(SHA1::from_str(&objs[0]).unwrap())
                }
            }
        }
    };

    !todo!()
}