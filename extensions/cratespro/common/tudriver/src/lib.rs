pub mod tugraph_client;

#[cfg(test)]
mod tests {

    use neo4rs::*;
    use std::env;

    /// This is the test to test whether the Tugraph is setup.
    #[tokio::test]
    async fn test_tugraph_setup() {
        let tugraph_bolt_url = env::var("TUGRAPH_BOLT_URL").unwrap();
        let tugraph_user_name = env::var("TUGRAPH_USER_NAME").unwrap();
        let tugraph_user_password = env::var("TUGRAPH_USER_PASSWORD").unwrap();

        // build bolt config
        let default_graph_config = ConfigBuilder::default()
            .uri(&tugraph_bolt_url)
            .user(&tugraph_user_name)
            .password(&tugraph_user_password)
            .db("default")
            .build()
            .unwrap();

        // connect the database
        let default_graph = Graph::connect(default_graph_config).await.unwrap();

        let _ = default_graph
            .run(query(
                "CALL dbms.graph.createGraph('graph_for_test', 'description', 2045)",
            ))
            .await;

        let config = ConfigBuilder::default()
            .uri(&tugraph_bolt_url)
            .user(&tugraph_user_name)
            .password(&tugraph_user_password)
            .db("graph_for_test")
            .build()
            .unwrap();

        let graph = Graph::connect(config).await.unwrap();

        graph.run(query("CALL db.dropDB()")).await.unwrap();
        graph.run(query("CALL db.createVertexLabel('person', 'id' , 'id' ,INT32, false, 'name' ,STRING, false)")).await.unwrap();
        graph
            .run(query(
                "CALL db.createEdgeLabel('is_friend','[[\"person\",\"person\"]]')",
            ))
            .await
            .unwrap();
        graph
            .run(query(
                "create (n1:person {name:'jack',id:1}), (n2:person {name:'lucy',id:2})",
            ))
            .await
            .unwrap();
        graph
            .run(query(
                "match (n1:person {id:1}), (n2:person {id:2}) create (n1)-[r:is_friend]->(n2)",
            ))
            .await
            .unwrap();
        let mut result = graph
            .execute(query("match (n)-[r]->(m) return n,r,m"))
            .await
            .unwrap();

        if let Ok(Some(row)) = result.next().await {
            let n: Node = row.get("n").unwrap();
            assert_eq!(n.id(), 0);
            let r: Relation = row.get("r").unwrap();
            assert_eq!(r.start_node_id(), 0);
            assert_eq!(r.end_node_id(), 1);
            let m: Node = row.get("m").unwrap();
            assert_eq!(m.id(), 1);
        } else {
            panic!("Error no result");
        }
    }
}
