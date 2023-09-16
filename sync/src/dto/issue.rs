use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueEventDto{
    action: String,
    issue: Issue,
    repository: Repository,
    sender: User,
}



#[derive(Serialize, Deserialize, Debug)]
pub struct Issue{
    id: u64,
    number: u64,
    title: String,
    user: User,
    state: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct User{
    login: String,
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Repository{
    id: u64,
    name: String,
    full_name: String,
}