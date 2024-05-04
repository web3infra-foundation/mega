use clap::Parser;
use sea_orm::{ActiveModelTrait, DbConn, Set};
use venus::{hash::SHA1, internal::object::types::ObjectType};

use crate::{
    command::branch,
    db,
    model::reference::{self, ActiveModel},
    utils::util,
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
    // TODO use restore to change the working directory
    unimplemented!("restore to change the working directory");

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
    // TODO use restore to change the working directory
    unimplemented!("restore to change the working directory");
    // update HEAD
    let mut head: ActiveModel = reference::Model::current_head(db).await.unwrap().into();

    head.name = Set(Some(branch_name));
    head.commit = Set(None);
    head.save(db).await.unwrap();
}
