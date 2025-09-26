use std::collections::HashMap;

use cedar_policy::{Entities, Schema};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};

use crate::{
    objects::{Issue, ChangeList, Repo, User, UserGroup},
    util::EntityUid,
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EntityStore {
    users: HashMap<EntityUid, User>,
    repos: HashMap<EntityUid, Repo>,
    change_lists: HashMap<EntityUid, ChangeList>,
    issues: HashMap<EntityUid, Issue>,
    user_groups: HashMap<EntityUid, UserGroup>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            repos: HashMap::new(),
            change_lists: HashMap::new(),
            issues: HashMap::new(),
            user_groups: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn as_entities(&self, schema: &Schema) -> Entities {
        let users = self.users.values().map(|user| user.clone().into());
        let repos = self.repos.values().map(|repo| repo.clone().into());
        let merge_requests = self.change_lists.values().map(|user| user.clone().into());
        let issues = self.issues.values().map(|repo| repo.clone().into());
        let user_groups = self.user_groups.values().map(|group| group.clone().into());
        let all = users
            .chain(repos)
            .chain(user_groups)
            .chain(merge_requests)
            .chain(issues);
        Entities::from_entities(all, Some(schema)).unwrap()
    }

    pub fn merge(&mut self, other: EntityStore) {
        self.users.extend(other.users);
        self.repos.extend(other.repos);
        self.change_lists.extend(other.change_lists);
        self.issues.extend(other.issues);
        self.user_groups.extend(other.user_groups);
    }
}

pub fn generate_entity(user: &str, repo: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut json_data = json!({
        "users": {
        },
        "repos": {
        },
        "user_groups": {
            "UserGroup::\"admin\"": {
                "euid": "UserGroup::\"admin\"",
                "parents": [
                    "UserGroup::\"matainer\""
                ]
            },
            "UserGroup::\"matainer\"": {
                "euid": "UserGroup::\"matainer\"",
                "parents": [
                    "UserGroup::\"reader\""
                ]
            },
            "UserGroup::\"reader\"": {
                "euid": "UserGroup::\"reader\"",
                "parents": []
            }
        },
        "merge_requests": {
        },
        "issues": {
        }
    });

    if let Some(users) = json_data.get_mut("users")
        && let Some(users_map) = users.as_object_mut()
    {
        users_map.insert(
            format!("User::\"{user}\""),
            json!({
                    "euid": format!("User::\"{user}\""),
                    "parents": [
                        "UserGroup::\"admin\""
                    ]
            }),
        );
    }

    if let Some(repos) = json_data.get_mut("repos")
        && let Some(repos_map) = repos.as_object_mut()
    {
        repos_map.insert(
            format!("Repository::\"{repo}\""),
            json!({
                    "euid": format!("Repository::\"{repo}\""),
                    "is_private": true,
                    "admins": "UserGroup::\"admin\"",
                    "maintainers": "UserGroup::\"matainer\"",
                    "readers": "UserGroup::\"reader\"",
                    "parents": []
            }),
        );
    }
    Ok(to_string_pretty(&json_data)?)
}
