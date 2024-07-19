use context::AppContext;

mod context;

fn main() {
    tracing_subscriber::fmt().pretty().init();
    let current_dir = env!("CARGO_MANIFEST_DIR");
    let (schema_path, policies_path) = (
        format!("{}/{}", current_dir, "mega.cedarschema"),
        format!("{}/{}", current_dir, "mega_policies.cedar"),
    );

    match AppContext::new("./entities.json", schema_path, policies_path) {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("Failed to load entities, policies, or schema: {e}");
            std::process::exit(1);
        }
    };
}

#[cfg(test)]
mod test {
    use cedar_policy::*;

    #[test]
    fn test_cedar() {
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
}
