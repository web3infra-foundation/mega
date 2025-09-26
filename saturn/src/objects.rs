use std::collections::HashSet;

use cedar_policy::{Entity, RestrictedExpression};
use serde::{Deserialize, Serialize};

use crate::util::EntityUid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    euid: EntityUid,
    parents: HashSet<EntityUid>,
}

impl From<User> for Entity {
    fn from(value: User) -> Entity {
        Entity::new_no_attrs(
            value.euid.into(),
            value.parents.into_iter().map(|euid| euid.into()).collect(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroup {
    euid: EntityUid,
    parents: HashSet<EntityUid>,
}

impl From<UserGroup> for Entity {
    fn from(value: UserGroup) -> Entity {
        Entity::new_no_attrs(
            value.euid.into(),
            value.parents.into_iter().map(|euid| euid.into()).collect(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    euid: EntityUid,
    is_private: bool,
    admins: EntityUid,
    maintainers: EntityUid,
    readers: EntityUid,
    parents: HashSet<EntityUid>,
}

impl From<Repo> for Entity {
    fn from(value: Repo) -> Self {
        let attrs = [
            (
                "is_private",
                RestrictedExpression::new_bool(value.is_private),
            ),
            (
                "admins",
                format!("{}", value.admins.as_ref()).parse().unwrap(),
            ),
            (
                "maintainers",
                format!("{}", value.maintainers.as_ref()).parse().unwrap(),
            ),
            (
                "readers",
                format!("{}", value.readers.as_ref()).parse().unwrap(),
            ),
        ]
        .into_iter()
        .map(|(x, v)| (x.into(), v))
        .collect();

        let parents = value.parents.into_iter().map(|euid| euid.into()).collect();

        Entity::new(value.euid.into(), attrs, parents).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeList {
    euid: EntityUid,
    repo: EntityUid,
    parents: HashSet<EntityUid>,
}

impl From<ChangeList> for Entity {
    fn from(value: ChangeList) -> Entity {
        let attrs = [("repo", format!("{}", value.repo.as_ref()).parse().unwrap())]
            .into_iter()
            .map(|(x, v)| (x.into(), v))
            .collect();

        Entity::new(
            value.euid.into(),
            attrs,
            value.parents.into_iter().map(|euid| euid.into()).collect(),
        )
        .unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    euid: EntityUid,
    repo: EntityUid,
    parents: HashSet<EntityUid>,
}

impl From<Issue> for Entity {
    fn from(value: Issue) -> Entity {
        let attrs = [("repo", format!("{}", value.repo.as_ref()).parse().unwrap())]
            .into_iter()
            .map(|(x, v)| (x.into(), v))
            .collect();

        Entity::new(
            value.euid.into(),
            attrs,
            value.parents.into_iter().map(|euid| euid.into()).collect(),
        )
        .unwrap()
    }
}
