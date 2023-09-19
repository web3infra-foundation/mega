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
    id: i64,
    number: i64,
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
    id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Repository{
    id: i64,
    name: String,
    full_name: String,
}

impl IssueEventDto{
    pub fn convert_to_model(&self) -> issue::ActiveModel {
        let closed_at = if let Some(s) = &self.issue.closed_at{
            Some(NaiveDateTime::parse_from_str(s.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap())
        }
        else {
            None
        };
        issue::ActiveModel{
            id: Set(self.issue.id),
            number: Set(self.issue.number),
            title: Set(self.issue.title.clone()),
            sender_name: Set(self.sender.login.clone()),
            sender_id: Set(self.sender.id),
            state: Set(self.action.clone()),
            created_at: Set(NaiveDateTime::parse_from_str(self.issue.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            updated_at: Set(NaiveDateTime::parse_from_str(self.issue.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            closed_at: Set(closed_at),
            repo_path: Set(self.repository.full_name.clone()),
            repo_id: Set(self.repository.id),
        }
    }

    pub fn id(&self) -> i64{
        self.issue.id
    }

    pub fn action(&self) -> &String{
        &self.action
    }
}

pub fn convert_model_to_dto(issue: &issue::ActiveModel) -> Issue{
    let closed_at = if let Some(date_time) = issue.closed_at.clone().unwrap(){
        Some(date_time.to_string())
    }
    else {
        None
    };
    Issue{
        id: issue.id.clone().unwrap(),
        number: issue.number.clone().unwrap(),
        title: issue.title.clone().unwrap(),
        state: issue.state.clone().unwrap(),
        user: User{
            login: issue.sender_name.clone().unwrap(),
            id: issue.sender_id.clone().unwrap(),
        },
        created_at: issue.created_at.clone().unwrap().to_string(),
        updated_at: issue.updated_at.clone().unwrap().to_string(),
        closed_at: closed_at,
    }
}