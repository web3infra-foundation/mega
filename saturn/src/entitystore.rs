use std::collections::HashMap;

use cedar_policy::{Entities, Schema};
use serde::{Deserialize, Serialize};

use crate::{
    objects::{Issue, MergeRequest, Repo, User, UserGroup},
    util::EntityUid,
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EntityStore {
    users: HashMap<EntityUid, User>,
    repos: HashMap<EntityUid, Repo>,
    merge_requests: HashMap<EntityUid, MergeRequest>,
    issues: HashMap<EntityUid, Issue>,
    user_groups: HashMap<EntityUid, UserGroup>,
}

impl EntityStore {
    #[allow(dead_code)]
    pub fn as_entities(&self, schema: &Schema) -> Entities {
        let users = self.users.values().map(|user| user.clone().into());
        let repos = self.repos.values().map(|repo| repo.clone().into());
        let merge_requests = self.merge_requests.values().map(|user| user.clone().into());
        let issues = self.issues.values().map(|repo| repo.clone().into());
        let user_groups = self.user_groups.values().map(|group| group.clone().into());
        let all = users
            .chain(repos)
            .chain(user_groups)
            .chain(merge_requests)
            .chain(issues);
        Entities::from_entities(all, Some(schema)).unwrap()
    }

    #[allow(dead_code)]
    pub fn merge(&mut self, other: EntityStore) {
        self.users.extend(other.users);
        self.repos.extend(other.repos);
        self.merge_requests.extend(other.merge_requests);
        self.issues.extend(other.issues);
        self.user_groups.extend(other.user_groups);
    }
}
