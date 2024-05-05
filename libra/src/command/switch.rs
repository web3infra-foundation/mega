use std::str::FromStr;

use clap::Parser;
use sea_orm::{ActiveModelTrait, DbConn, Set};
use venus::{hash::SHA1, internal::object::types::ObjectType};

use crate::{
    command::branch,
    db,
    model::reference::{self, ActiveModel},
    utils::util,
};

use super::{
    restore::{self, RestoreArgs},
    status,
};

#[derive(Parser, Debug)]
pub struct SwitchArgs {
    #[clap(required_unless_present("create"), required_unless_present("detach"))]
    branch: Option<String>,

    #[clap(long, short, group = "sub")]
    create: Option<String>,

    //available only with create
    #[clap(requires = "create")]
    create_base: Option<String>,

    #[clap(long, short, action, default_value = "false", group = "sub")]
    detach: bool,
}

fn get_commit_base(commit_base: &str) -> Result<SHA1, String> {
    let storage = util::objects_storage();

    let commits = storage.search(commit_base);
    if commits.is_empty() {
        return Err(format!("fatal: invalid reference: {}", commit_base));
    } else if commits.len() > 1 {
        return Err(format!("fatal: ambiguous argument: {}", commit_base));
    }
    if storage.is_object_type(&commits[0], ObjectType::Commit) {
        Err(format!("fatal: reference is not a commit: {}", commit_base))
    } else {
        Ok(commits[0])
    }
}

pub async fn execute(args: SwitchArgs) {
    // check status
    let unstaged = status::changes_to_be_staged();
    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        status::execute().await;
        eprintln!("fatal: uncommitted changes, can't switch branch");
        return;
    } else if !status::changes_to_be_committed().await.is_empty() {
        status::execute().await;
        eprintln!("fatal: unstaged changes, can't switch branch");
        return;
    }

    let db = db::get_db_conn().await.unwrap();
    match args.create {
        Some(new_branch_name) => {
            branch::create_branch(new_branch_name.clone(), args.create_base).await;
            switch_to_branch(&db, new_branch_name).await;
        }
        None => match args.detach {
            true => {
                let commit_base = get_commit_base(&args.branch.unwrap());
                if commit_base.is_err() {
                    eprintln!("{}", commit_base.unwrap());
                    return;
                }
                switch_to_commit(&db, commit_base.unwrap()).await;
            }
            false => {
                switch_to_branch(&db, args.branch.unwrap()).await;
            }
        },
    }
}

/// change the working directory to the version of commit_hash
async fn switch_to_commit(db: &DbConn, commit_hash: SHA1) {
    restore_to_commit(commit_hash).await;
    // update HEAD
    let mut head: ActiveModel = reference::Model::current_head(db).await.unwrap().into();
    head.name = Set(None);
    head.commit = Set(Some(commit_hash.to_string()));
    head.save(db).await.unwrap();
}

async fn switch_to_branch(db: &DbConn, branch_name: String) {
    let target_branch = reference::Model::find_branch_by_name(db, &branch_name)
        .await
        .unwrap();
    if target_branch.is_none() {
        eprintln!("fatal: branch '{}' not found", &branch_name);
        return;
    }
    let commit_id = target_branch.unwrap().commit.unwrap();
    let commit_id = SHA1::from_str(&commit_id).unwrap();
    restore_to_commit(commit_id).await;
    // update HEAD
    let mut head: ActiveModel = reference::Model::current_head(db).await.unwrap().into();

    head.name = Set(Some(branch_name));
    head.commit = Set(None);
    head.save(db).await.unwrap();
}

async fn restore_to_commit(commit_id: SHA1) {
    // TODO may wrong
    let restore_args = RestoreArgs {
        worktree: true,
        staged: true,
        source: Some(commit_id.to_plain_str()),
        pathspec: vec![util::working_dir_string()],
    };
    restore::execute(restore_args).await;
}


#[cfg(test)]
mod tests {
    use std::env;
    use crate::command::restore::RestoreArgs;
    use crate::utils::{test, util};
    use super::*;
    #[test]
    fn test_parse_from() {
        env::set_current_dir("./libra_test_repo").unwrap();
        let commit_id = SHA1::from_str("0cb5eb6281e1c0df48a70716869686c694706189").unwrap();
        let restore_args = RestoreArgs::parse_from([
            "restore", // important, the first will be ignored
            "--worktree",
            "--staged",
            "--source",
            &commit_id.to_plain_str(),
            util::working_dir().to_str().unwrap(),
        ]);
        println!("{:?}", restore_args);
    }

}