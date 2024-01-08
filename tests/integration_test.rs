use crate::common_test::P2pTestConfig;
mod common_test;

#[tokio::test]
#[ignore]
async fn test_peovide_and_clone_e2e() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/mega_p2p/compose.yaml".to_string(),
        lifecycle_url: "http://localhost:8301/api/v1/status".to_string(),
        lifecycle_retrying: 5,
    };
    common_test::init_p2p_server(init_config.clone()).await;
    common_test::provide_data_before_test();
    test_mega_provide().await;
    test_mega_clone().await;
    test_mega_clone_obj().await;
    common_test::stop_p2p_server(init_config);
}

async fn test_mega_provide() {
    let client = reqwest::Client::new();
    let resp = client
        .put("http://localhost:8301/api/v1/mega/provide?repo_name=mega.git")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

async fn test_mega_clone() {
    //note that if secret of nodeA in compose file has been change, should also update the perrid in the below link
    let resp = reqwest::get("http://localhost:8401/api/v1/mega/clone?mega_address=p2p://16Uiu2HAmCpKDLiX1NK6ULnYycq88jqaptNMRo1f4mRSu3VqHMry1/mega.git")
    .await.unwrap();
    assert_eq!(resp.status(), 200);
}

async fn test_mega_clone_obj() {
    let resp = reqwest::get("http://localhost:8501/api/v1/mega/clone-obj?mega_address=p2p://16Uiu2HAmCpKDLiX1NK6ULnYycq88jqaptNMRo1f4mRSu3VqHMry1/mega.git")
    .await.unwrap();
    assert_eq!(resp.status(), 200);
}
