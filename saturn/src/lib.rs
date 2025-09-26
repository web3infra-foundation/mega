use std::fmt::{self, Display};

pub mod context;
pub mod entitystore;
mod objects;
pub mod util;

pub enum ActionEnum {
    // ** Anyone
    // ViewRepo,
    // PullRepo,
    // ForkRepo,
    // PushRepo,
    // OpenIssue,
    // ** Maintainer
    CreateChangeList,
    EditIssue,
    EditChangeList,
    AssignIssue,
    ApproveChangeList,
    // ** Admin
    AddMaintainer,
    AddAdmin,
    DeleteRepo,
    DeleteIssue,
    DeleteChangeList,
}

impl Display for ActionEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ActionEnum::CreateChangeList => "createChangeList",
            ActionEnum::EditIssue => "editIssue",
            ActionEnum::EditChangeList => "editChangeList",
            ActionEnum::AssignIssue => "assignIssue",
            ActionEnum::ApproveChangeList => "approveChangeList",
            ActionEnum::AddMaintainer => "addMaintainer",
            ActionEnum::AddAdmin => "addAdmin",
            ActionEnum::DeleteRepo => "deleteRepo",
            ActionEnum::DeleteIssue => "deleteIssue",
            ActionEnum::DeleteChangeList => "deleteChangeList",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod test {
    use std::{fs, sync::Once};

    use cedar_policy::{Authorizer, Context, Entities, PolicySet, Request};

    use crate::{
        context::{CedarContext, Error},
        entitystore::EntityStore,
        util::EntityUid,
    };

    static INIT: Once = Once::new();

    fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt().pretty().init();
        });
    }

    #[test]
    fn test_without_entity() {
        const POLICY_SRC: &str = r#"
    permit(principal == User::"alice", action == Action::"view", resource == File::"93");
    "#;
        let policy: PolicySet = POLICY_SRC.parse().unwrap();

        let action = r#"Action::"view""#.parse().unwrap();
        let alice = r#"User::"alice""#.parse().unwrap();
        let file = r#"File::"93""#.parse().unwrap();
        let request = Request::new(alice, action, file, Context::empty(), None).unwrap();

        let entities = Entities::empty();
        let authorizer = Authorizer::new();
        let answer = authorizer.is_authorized(&request, &policy, &entities);

        // Should output `Allow`
        println!("{:?}", answer.decision());

        let action = r#"Action::"view""#.parse().unwrap();
        let bob = r#"User::"bob""#.parse().unwrap();
        let file = r#"File::"93""#.parse().unwrap();
        let request = Request::new(bob, action, file, Context::empty(), None).unwrap();

        let answer = authorizer.is_authorized(&request, &policy, &entities);

        // Should output `Deny`
        println!("{:?}", answer.decision());
    }

    fn load_context(entities: EntityStore) -> CedarContext {
        CedarContext::new(entities).unwrap()
    }

    #[test]
    fn test_project_path_policy() {
        init_tracing();
        let entities_path = "./test/project/.mega.json";
        let entities_file = fs::File::open(entities_path).unwrap();
        let entities = serde_json::from_reader(entities_file).unwrap();

        let app_context = load_context(entities);
        let admin: EntityUid = r#"User::"benjamin.747""#.parse().unwrap();
        let maintainer: EntityUid = r#"User::"besscroft""#.parse().unwrap();
        let anyone: EntityUid = r#"User::"anyone""#.parse().unwrap();
        let resource: EntityUid = r#"Repository::"project""#.parse().unwrap();

        // admin can view repo
        assert!(
            app_context
                .is_authorized(
                    &admin,
                    r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_ok()
        );
        // admin can delete repo
        assert!(
            app_context
                .is_authorized(
                    &admin,
                    r#"Action::"deleteRepo""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_ok()
        );

        // anyone can view public_repo
        assert!(
            app_context
                .is_authorized(
                    &anyone,
                    r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_ok()
        );

        assert!(
            app_context
                .is_authorized(
                    &anyone,
                    r#"Action::"openIssue""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty(),
                )
                .is_ok()
        );

        // normal user can't assign issue
        assert!(
            app_context
                .is_authorized(
                    &anyone,
                    r#"Action::"assignIssue""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_err_and(|e| matches!(e, Error::AuthDenied(_)))
        );
        assert!(
            app_context
                .is_authorized(
                    &anyone,
                    r#"Action::"approveMergeRequest""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_err_and(|e| matches!(e, Error::AuthDenied(_)))
        );

        assert!(
            app_context
                .is_authorized(
                    &maintainer,
                    r#"Action::"approveMergeRequest""#.parse::<EntityUid>().unwrap(),
                    &resource,
                    Context::empty()
                )
                .is_ok()
        );
    }

    #[test]
    fn test_private_path_policy() {
        init_tracing();
        let parent_entities_file = fs::File::open("./test/project/.mega.json").unwrap();
        let parent_entities: EntityStore = serde_json::from_reader(parent_entities_file).unwrap();

        let entities_file = fs::File::open("./test/project/private/.mega.json").unwrap();
        let mut entities: EntityStore = serde_json::from_reader(entities_file).unwrap();

        entities.merge(parent_entities);

        let app_context = load_context(entities);
        let p_admin: EntityUid = r#"User::"benjamin.747""#.parse().unwrap();
        let admin: EntityUid = r#"User::"private""#.parse().unwrap();
        let anyone: EntityUid = r#"User::"anyone""#.parse().unwrap();
        let private_project: EntityUid = r#"Repository::"/project/bens_private""#.parse().unwrap();

        // admin under project should also have permisisons
        assert!(
            app_context
                .is_authorized(
                    &p_admin,
                    r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                    &private_project,
                    Context::empty()
                )
                .is_ok()
        );

        assert!(
            app_context
                .is_authorized(
                    &admin,
                    r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                    &private_project,
                    Context::empty()
                )
                .is_ok()
        );

        // not public, should deny
        assert!(
            app_context
                .is_authorized(
                    &anyone,
                    r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                    &private_project,
                    Context::empty()
                )
                .is_err_and(|e| matches!(e, Error::AuthDenied(_)))
        );
    }
}
