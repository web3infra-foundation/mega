use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "List, create, or delete branches")]
pub struct BranchArgs {
    /// new branch name
    #[clap(group = "sub")]
    new_branch: Option<String>,

    /// base branch name or commit hash
    #[clap(requires = "new_branch")]
    commit_hash: Option<String>,

    /// list all branches
    #[clap(short, long, action, group = "sub", default_value = "true")]
    list: bool,

    /// force delete branch
    #[clap(short = 'D', long, group = "sub")]
    delete: Option<String>,

    /// show current branch
    #[clap(long, action, group = "sub")]
    show_curren: bool,
}
pub async fn execute(args: BranchArgs) {
    if args.new_branch.is_some() {
        todo!();
    } else if args.delete.is_some() {
        delete_branch(args.delete.unwrap()).await;
    } else if args.show_curren {
        show_current_branch().await;
    } else if args.list {
        // 兜底list
        list_branches().await;
    } else {
        panic!("should not reach here")
    }
}

async fn create_branch(new_branch: String, commit_hash: String) {
    unimplemented!();
}

async fn delete_branch(branch_name: String) {
    unimplemented!();
}

async fn show_current_branch() {
    unimplemented!();
}

async fn list_branches() {
    unimplemented!();
}
