use cedar_policy::Context;
use context::AppContext;
use util::EntityUid;

mod context;
mod entitystore;
mod objects;
mod util;

fn main() {
    tracing_subscriber::fmt().pretty().init();
    let current_dir = env!("CARGO_MANIFEST_DIR");
    let (schema_path, policies_path, entities_path) = (
        format!("{}/{}", current_dir, "mega.cedarschema"),
        format!("{}/{}", current_dir, "mega_policies.cedar"),
        format!("{}/{}", current_dir, "./entities.json"),
    );

    let app_context = match AppContext::new(entities_path, schema_path, policies_path) {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("Failed to load entities, policies, or schema: {e}");
            std::process::exit(1);
        }
    };
    let anyone: EntityUid = r#"User::"anyone""#.parse().unwrap();
    let resource: EntityUid = r#"Repository::"public""#.parse().unwrap();
    // anyone can view public_repo
    assert!(app_context
        .is_authorized(
            anyone,
            r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
            resource.clone(),
            Context::empty()
        )
        .is_ok());
}

#[cfg(test)]
mod test {
    use cedar_policy::{Authorizer, Context, Entities, PolicySet, Request};

    use crate::{context::AppContext, util::EntityUid};

    #[test]
    fn test_origin_policy() {
        const POLICY_SRC: &str = r#"
    permit(principal == User::"alice", action == Action::"view", resource == File::"93");
    "#;
        let policy: PolicySet = POLICY_SRC.parse().unwrap();

        let action = r#"Action::"view""#.parse().unwrap();
        let alice = r#"User::"alice""#.parse().unwrap();
        let file = r#"File::"93""#.parse().unwrap();
        let request = Request::new(
            Some(alice),
            Some(action),
            Some(file),
            Context::empty(),
            None,
        )
        .unwrap();

        let entities = Entities::empty();
        let authorizer = Authorizer::new();
        let answer = authorizer.is_authorized(&request, &policy, &entities);

        // Should output `Allow`
        println!("{:?}", answer.decision());

        let action = r#"Action::"view""#.parse().unwrap();
        let bob = r#"User::"bob""#.parse().unwrap();
        let file = r#"File::"93""#.parse().unwrap();
        let request =
            Request::new(Some(bob), Some(action), Some(file), Context::empty(), None).unwrap();

        let answer = authorizer.is_authorized(&request, &policy, &entities);

        // Should output `Deny`
        println!("{:?}", answer.decision());
    }

    fn load_context() -> AppContext {
        tracing_subscriber::fmt().pretty().init();
        AppContext::new(
            "./entities.json",
            "./mega.cedarschema",
            "./mega_policies.cedar",
        )
        .unwrap()
    }
    #[test]
    fn test_admin_policy() {
        let app_context = load_context();
        let principal: EntityUid = r#"User::"kesha""#.parse().unwrap();
        let resource: EntityUid = r#"Repository::"mega""#.parse().unwrap();

        // admin can view repo
        assert!(app_context
            .is_authorized(
                principal.clone(),
                r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                resource.clone(),
                Context::empty()
            )
            .is_ok());
        // admin can delete repo
        assert!(app_context
            .is_authorized(
                principal,
                r#"Action::"deleteRepo""#.parse::<EntityUid>().unwrap(),
                resource.clone(),
                Context::empty()
            )
            .is_ok());
    }

    #[test]
    fn test_anyone_policy() {
        let app_context = load_context();
        let anyone: EntityUid = r#"User::"anyone""#.parse().unwrap();
        let resource: EntityUid = r#"Repository::"public""#.parse().unwrap();
        // anyone can view public_repo
        assert!(app_context
            .is_authorized(
                anyone.clone(),
                r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                resource.clone(),
                Context::empty()
            )
            .is_ok());
        // anyone can not view mega
        assert!(app_context
            .is_authorized(
                anyone,
                r#"Action::"viewRepo""#.parse::<EntityUid>().unwrap(),
                r#"Repository::"mega""#.parse::<EntityUid>().unwrap(),
                Context::empty()
            )
            .is_err());
    }
}
