use std::{
    env,
    io::Write,
    path::{Path, PathBuf},
};

use crate::common_test::P2pTestConfig;
use git::internal::pack::counter::GitTypeCounter;
use git2::{Repository, Signature};
use go_defer::defer;

mod common_test;

#[tokio::test]
#[ignore]
async fn test_p2p_basic() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/mega_p2p/compose.yaml".to_string(),
        pack_path: "tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack"
            .to_string(),
        lifecycle_url: "http://localhost:8301/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_path: "projects/test-pack".to_string(),
        commit_id: "d50df695086eea6253a237cb5ac44af1629e7ced".to_string(),
        sub_commit_id: "31fbd13995ef1acc920294f6a170ce8d05abd665".to_string(),
        counter: GitTypeCounter::default(),
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
        pack_path: "tests/data/packs/mega-b5d9805f1d67ad2d5e2f0cd183ca54f14d142874.pack"
            .to_string(),
        lifecycle_url: "http://localhost:8000/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_path: "/projects/testmega".to_string(),
        commit_id: "b5d9805f1d67ad2d5e2f0cd183ca54f14d142874".to_string(),
        sub_commit_id: "31fbd13995ef1acc920294f6a170ce8d05abd665".to_string(),
        // counter: GitTypeCounter { commit: 37, tree: 154, blob: 133, tag: 0, ofs_delta: 0, ref_delta: 0 },
        counter: GitTypeCounter {
            commit: 610,
            tree: 2117,
            blob: 1860,
            tag: 0,
            ofs_delta: 0,
            ref_delta: 0,
        },
    };
    defer!(
        common_test::stop_server(&init_config);
    );
    // common_test::build_image(&init_config);
    common_test::start_server(&init_config);
    common_test::lifecycle_check(&init_config).await;
    common_test::init_by_pack(&init_config).await;
    check_obj_nums(&init_config).await;
    test_http_clone_all(&init_config).await;
    test_http_clone_sub_dir(&init_config).await;
    test_update_and_push(&init_config);
}

async fn check_obj_nums(config: &P2pTestConfig) {
    let client = reqwest::Client::new();
    let repo_count_api = format!(
        "http://localhost:8000/api/v1/count-objs?repo_path={}",
        config.repo_path
    );
    let check_res: GitTypeCounter = client
        .get(repo_count_api)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(check_res, config.counter);
}

async fn test_http_clone_all(config: &P2pTestConfig) {
    let repo_name = Path::new(&config.repo_path).file_name().unwrap();
    let into_path = PathBuf::from("/tmp/.mega").join(repo_name);
    let url = format!("http://localhost:8000{}.git", config.repo_path);
    common_test::git2_clone(&url, into_path.to_str().unwrap());
    let last_id = common_test::get_last_commit_id(into_path.to_str().unwrap()).to_string();
    assert_eq!(last_id, config.commit_id)
}

async fn test_http_clone_sub_dir(config: &P2pTestConfig) {
    let into_path = PathBuf::from("/tmp/.mega").join("src");
    let url = format!("http://localhost:8000{}/src.git", config.repo_path);
    common_test::git2_clone(&url, into_path.to_str().unwrap());
    let last_id = common_test::get_last_commit_id(into_path.to_str().unwrap()).to_string();
    assert_eq!(last_id, config.sub_commit_id)
}

fn test_update_and_push(config: &P2pTestConfig) {
    let repo_name = Path::new(&config.repo_path).file_name().unwrap();
    let repo_path = PathBuf::from("/tmp/.mega").join(repo_name);
    env::set_current_dir(&repo_path).unwrap();
    let file_path = Path::new("newfile.txt");
    let mut file = std::fs::File::create(file_path).unwrap();
    file.write_all(b"This is a new file created by mega integration test")
        .unwrap();

    // add file to the index
    let repo = Repository::open(&repo_path).expect("Failed to open repository");
    let mut index = repo.index().unwrap();
    index.add_path(file_path).unwrap();
    index.write().unwrap();

    // Commit the changes
    let head = repo.head().unwrap();
    let head_commit = repo.find_commit(head.target().unwrap()).unwrap();
    let signature = Signature::now("Mega", "your.email@example.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let commit_id = repo
        .commit(
            Some("HEAD"), // point HEAD to the new commit
            &signature,
            &signature,
            "Mega Test Commit",
            &tree,
            &[&head_commit],
        )
        .unwrap();

    // Push the commit to a remote branch
    let mut remote = repo.find_remote("origin").unwrap();
    let refspecs = ["refs/heads/master:refs/heads/master"];
    remote.push(&refspecs, None).unwrap();

    // chcek cloned project's commit id
    let url = format!("http://localhost:8000{}.git", config.repo_path);
    let repo_copy = Repository::clone(&url, PathBuf::from("/tmp/.mega/copy")).unwrap();
    let head = repo_copy.head().unwrap().target().unwrap();
    assert_eq!(head, commit_id)
}
