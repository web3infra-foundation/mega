use crate::common_test::P2pTestConfig;
use go_defer::defer;

mod common_test;

#[tokio::test]
#[ignore]
async fn test_p2p_basic() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/mega_p2p/compose.yaml".to_string(),
        pack_path: "tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack".to_string(),
        lifecycle_url: "http://localhost:8301/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_name: "git".to_string(),
        commit_id: "d50df695086eea6253a237cb5ac44af1629e7ced".to_string(),
        obj_num: 0
    };
    defer!(
        common_test::stop_server(&init_config);
    );
    common_test::start_server(&init_config);
    common_test::lifecycle_check(&init_config).await;
    common_test::init_by_pack(&init_config).await;
    test_mega_provide().await;
    test_mega_clone().await;
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
    //note that if secret of nodeA in compose file has been change, peerid in the below link should also be updated
    let resp = reqwest::get("http://localhost:8401/api/v1/mega/clone?mega_address=p2p://16Uiu2HAmCpKDLiX1NK6ULnYycq88jqaptNMRo1f4mRSu3VqHMry1/mega.git").await.unwrap();
    assert_eq!(resp.status(), 200);
}

// async fn test_mega_clone_obj() {
//     let resp = reqwest::get("http://localhost:8501/api/v1/mega/clone-obj?mega_address=p2p://16Uiu2HAmCpKDLiX1NK6ULnYycq88jqaptNMRo1f4mRSu3VqHMry1/mega.git").await.unwrap();
//     assert_eq!(resp.status(), 200);
// }

#[tokio::test]
#[ignore]
async fn test_http() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/http/compose.yaml".to_string(),
        pack_path: "tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack".to_string(),
        lifecycle_url: "http://localhost:8000/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_name: "git".to_string(),
        commit_id: "d50df695086eea6253a237cb5ac44af1629e7ced".to_string(),
        obj_num: 10000
    };
    defer!(
        common_test::stop_server(&init_config);
    );
    common_test::build_image(&init_config);
    common_test::start_server(&init_config);
    common_test::lifecycle_check(&init_config).await;
    common_test::init_by_pack(&init_config).await;

}
