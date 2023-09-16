use serde::{Deserialize, Serialize};
use entity::issue;
use sea_orm::{ActiveValue::NotSet, Set};
use chrono::NaiveDateTime;

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

impl IssueEventDto{
    pub fn convert_to_model(&self, close_time: Option<String>) -> issue::ActiveModel {
        let closed_at = if let Some(s) = close_time{
            Some(NaiveDateTime::parse_from_str(self.issue.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap())
        }
        else {
            None
        };
        issue::ActiveModel{
            id: Set(self.issue.id),
            num: Set(self.issue.number),
            title: Set(self.issue.title.clone()),
            sender_id: Set(self.sender.login.clone()),
            state: Set(self.action.clone()),
            created_at: Set(NaiveDateTime::parse_from_str(self.issue.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            updated_at: Set(NaiveDateTime::parse_from_str(self.issue.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            closed_at: Set(closed_at),
            repo_path: Set(self.repository.full_name.clone()),
        }
    }
}