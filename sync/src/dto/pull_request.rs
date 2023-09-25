use serde::{Deserialize, Serialize};
use entity::pull_request;
use sea_orm::{Set};
use chrono::NaiveDateTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct PullRequestEventDto{
    action: String,
    pull_request: PullRequest,
    repository: Repository,
    sender: User,
}



#[derive(Serialize, Deserialize, Debug)]
pub struct PullRequest{
    id: i64,
    number: i64,
    title: String,
    user: User,
    state: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
    commits_url: String,
    patch_url: String,
    head: Commit,
    base: Commit,
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

#[derive(Serialize, Deserialize, Debug)]
struct Commit{
    label: String,
    r#ref: String,
}


impl PullRequestEventDto{
    pub fn convert_to_model(&self) -> pull_request::ActiveModel {
        let closed_at = self.pull_request.closed_at.as_ref()
            .map(|s| NaiveDateTime::parse_from_str(s.as_str(), "%Y-%m-%dT%H:%M:%SZ")
            .unwrap());
        let merged_at = self.pull_request.merged_at.as_ref()
            .map(|s| NaiveDateTime::parse_from_str(s.as_str(), "%Y-%m-%dT%H:%M:%SZ")
            .unwrap());
        pull_request::ActiveModel{
            id: Set(self.pull_request.id),
            number: Set(self.pull_request.number),
            title: Set(self.pull_request.title.clone()),
            sender_name: Set(self.sender.login.clone()),
            sender_id: Set(self.sender.id),
            state: Set(self.action.clone()),
            created_at: Set(NaiveDateTime::parse_from_str(self.pull_request.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            updated_at: Set(NaiveDateTime::parse_from_str(self.pull_request.created_at.as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap()),
            closed_at: Set(closed_at),
            merged_at: Set(merged_at),
            repo_path: Set(self.repository.full_name.clone()),
            repo_id: Set(self.repository.id),
            merge_commit_sha: Set(self.pull_request.merge_commit_sha.clone()),
            user_name: Set(self.pull_request.user.login.clone()),
            user_id: Set(self.pull_request.user.id),
            commits_url: Set(self.pull_request.commits_url.clone()),
            patch_url: Set(self.pull_request.patch_url.clone()),
            head_label: Set(self.pull_request.head.label.clone()),
            head_ref: Set(self.pull_request.head.r#ref.clone()),
            base_label: Set(self.pull_request.base.label.clone()),
            base_ref: Set(self.pull_request.base.r#ref.clone()),
        }
    }

    pub fn id(&self) -> i64{
        self.pull_request.id
    }

    pub fn action(&self) -> &String{
        &self.action
    }
}

pub fn convert_model_to_dto(pull_request: &pull_request::ActiveModel) -> PullRequest{
    let closed_at = pull_request.closed_at.clone()
        .unwrap()
        .map(|date_time| date_time.to_string());
    let merged_at = pull_request.merged_at.clone()
        .unwrap()
        .map(|date_time| date_time.to_string());
    PullRequest{
        id: pull_request.id.clone().unwrap(),
        number: pull_request.number.clone().unwrap(),
        title: pull_request.title.clone().unwrap(),
        state: pull_request.state.clone().unwrap(),
        user: User{
            login: pull_request.sender_name.clone().unwrap(),
            id: pull_request.sender_id.clone().unwrap(),
        },
        created_at: pull_request.created_at.clone().unwrap().to_string(),
        updated_at: pull_request.updated_at.clone().unwrap().to_string(),
        closed_at,
        merged_at,
        merge_commit_sha: pull_request.merge_commit_sha.clone().unwrap(),
        commits_url: pull_request.commits_url.clone().unwrap().to_string(),
        patch_url: pull_request.patch_url.clone().unwrap().to_string(),
        head: Commit { 
            label: pull_request.head_label.clone().unwrap().to_string(),
            r#ref: pull_request.head_ref.clone().unwrap().to_string()
            },
        base: Commit { 
            label: pull_request.base_label.clone().unwrap().to_string(),
            r#ref: pull_request.base_ref.clone().unwrap().to_string()
            },
    }
}