use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use git2::{Oid, Signature};
use go_defer::defer;

use ceres::protocol::TransportProtocol;

use crate::common::{P2pTestConfig, PackObjectIds};

mod common;

#[tokio::test]
#[ignore]
async fn test_p2p_basic() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/mega_p2p/compose.yaml".to_string(),
        pack_path: "tests/data/packs/pack-f8bbb573cef7d851957caceb491c073ee8e8de41.pack"
            .to_string(),
        lifecycle_url: "http://localhost:8301/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_path: "/projects/test-p2p".to_string(),
        commit_id: "f8bbb573cef7d851957caceb491c073ee8e8de41".to_string(),
        sub_commit_id: "3b7a920f971712ae657bc0ee194825f1327e1255".to_string(),
        // counter: GitTypeCounter::default(),
        clone_path: PathBuf::from("/tmp/.mega/integration_test"),
        protocol: TransportProtocol::P2p,
    };
    defer!(
        common::stop_server(&init_config);
    );
    common::start_server(&init_config);
    common::lifecycle_check(&init_config).await;
    common::init_by_pack(&init_config).await;
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
        pack_path: "tests/data/packs/pack-f8bbb573cef7d851957caceb491c073ee8e8de41.pack"
            .to_string(),
        lifecycle_url: "http://localhost:8000/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_path: "/projects/testhttp".to_string(),
        commit_id: "f8bbb573cef7d851957caceb491c073ee8e8de41".to_string(),
        sub_commit_id: "3b7a920f971712ae657bc0ee194825f1327e1255".to_string(),
        // counter: GitTypeCounter {
        //     commit: 612,
        //     tree: 2141,
        //     blob: 1873,
        //     tag: 0,
        //     ofs_delta: 0,
        //     ref_delta: 0,
        // },
        clone_path: PathBuf::from("/tmp/.mega/integration_test"),
        protocol: TransportProtocol::Http,
    };
    defer!(
        common::stop_server(&init_config);
    );
    // common_test::build_image(&init_config);
    common::start_server(&init_config);
    common::lifecycle_check(&init_config).await;
    common::init_by_pack(&init_config).await;
    // check_obj_nums(&init_config).await;
    test_clone_and_check_all_obj(&init_config).await;
    test_clone_sub_dir(&init_config).await;
    test_update_and_push(&init_config);
}

#[tokio::test]
#[ignore]
async fn test_ssh() {
    let init_config = P2pTestConfig {
        compose_path: "tests/compose/ssh/compose.yaml".to_string(),
        pack_path: "tests/data/packs/pack-f8bbb573cef7d851957caceb491c073ee8e8de41.pack"
            .to_string(),
        lifecycle_url: "http://localhost:8000/api/v1/status".to_string(),
        lifecycle_retrying: 5,
        repo_path: "/projects/testssh".to_string(),
        commit_id: "f8bbb573cef7d851957caceb491c073ee8e8de41".to_string(),
        sub_commit_id: "3b7a920f971712ae657bc0ee194825f1327e1255".to_string(),
        // counter: GitTypeCounter {
        //     commit: 612,
        //     tree: 2141,
        //     blob: 1873,
        //     tag: 0,
        //     ofs_delta: 0,
        //     ref_delta: 0,
        // },
        clone_path: PathBuf::from("/tmp/.mega/integration_test"),
        protocol: TransportProtocol::Ssh,
    };
    defer!(
        common::stop_server(&init_config);
    );
    common::start_server(&init_config);
    common::lifecycle_check(&init_config).await;
    common::init_by_pack(&init_config).await;
    // check_obj_nums(&init_config).await;
    test_clone_and_check_all_obj(&init_config).await;
    test_clone_sub_dir(&init_config).await;
    test_update_and_push(&init_config);
}

// async fn check_obj_nums(config: &P2pTestConfig) {
//     let client = reqwest::Client::new();
//     let repo_count_api = format!(
//         "http://localhost:8000/api/v1/count-objs?repo_path={}",
//         config.repo_path
//     );
//     let check_res: GitTypeCounter = client
//         .get(repo_count_api)
//         .send()
//         .await
//         .unwrap()
//         .json()
//         .await
//         .unwrap();
//     assert_eq!(check_res, config.counter);
// }

async fn test_clone_and_check_all_obj(config: &P2pTestConfig) {
    let repo_name = Path::new(&config.repo_path).file_name().unwrap();
    let into_path = config.clone_path.clone().join(repo_name);
    let url = repo_default_url(config);
    let repo = common::clone_by_type(config, &url, &into_path);
    defer!(
        std::fs::remove_dir_all(&into_path).unwrap();
    );
    let pack_ids: PackObjectIds = match fs::read_to_string(PathBuf::from(format!(
        "tests/data/pack-{}.toml",
        config.commit_id
    ))) {
        Ok(content) => toml::from_str(&content).unwrap(),
        Err(_) => panic!("read objectid toml error"),
    };
    for obj_id in pack_ids.commit_ids {
        let res = repo.find_commit(Oid::from_str(&obj_id).unwrap());
        assert!(res.is_ok(), "commit {} not exists", obj_id);
    }
    for obj_id in pack_ids.tree_ids {
        let res = repo.find_tree(Oid::from_str(&obj_id).unwrap());
        assert!(res.is_ok(), "tree {} not exists", obj_id);
    }
    for obj_id in pack_ids.blob_ids {
        let res = repo.find_blob(Oid::from_str(&obj_id).unwrap());
        assert!(res.is_ok(), "blob {} not exists", obj_id);
    }
    for obj_id in pack_ids.tag_ids {
        let res = repo.find_tag(Oid::from_str(&obj_id).unwrap());
        assert!(res.is_ok(), "tag {} not exists", obj_id);
    }
}

async fn test_clone_sub_dir(config: &P2pTestConfig) {
    let url = repo_sub_url(config);
    let into_path = config.clone_path.clone().join("src");
    let repo = common::clone_by_type(config, &url, &into_path);
    defer!(
        std::fs::remove_dir_all(&into_path).unwrap();
    );
    let head = repo.head().expect("Failed to get HEAD reference");
    let last_id = head.target().unwrap().to_string();
    assert_eq!(last_id, config.sub_commit_id)
}

fn test_update_and_push(config: &P2pTestConfig) {
    let repo_name = Path::new(&config.repo_path).file_name().unwrap();
    let into_patn = config.clone_path.clone().join(repo_name);
    let url = repo_default_url(config);
    let repo = common::clone_by_type(config, &url, &into_patn);
    defer!(
        std::fs::remove_dir_all(&into_patn).unwrap();
    );
    let relative_path = PathBuf::from("newfile.txt");
    let file_path = into_patn.clone().join(&relative_path);
    let mut file = std::fs::File::create(file_path).unwrap();
    file.write_all(b"This is a new file created by mega integration test")
        .unwrap();

    // add file to the index
    let mut index = repo.index().unwrap();
    index.add_path(&relative_path).unwrap();
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
    common::push_by_type(config, &repo);

    // chcek cloned project's commit id
    let copied_path = config.clone_path.clone().join("copy");
    let repo_copy = common::clone_by_type(config, &url, &copied_path);
    defer!(
        std::fs::remove_dir_all(&copied_path).unwrap();
    );
    let head = repo_copy.head().unwrap().target().unwrap();
    assert_eq!(head, commit_id)
}

fn repo_default_url(config: &P2pTestConfig) -> String {
    match config.protocol {
        TransportProtocol::Http => format!("http://localhost:8000{}.git", config.repo_path),
        TransportProtocol::Ssh => format!("ssh://git@localhost:8100{}.git", config.repo_path),
        _ => todo!(),
    }
}

fn repo_sub_url(config: &P2pTestConfig) -> String {
    match config.protocol {
        TransportProtocol::Http => format!("http://localhost:8000{}/src.git", config.repo_path),
        TransportProtocol::Ssh => format!("ssh://git@localhost:8100{}/src.git", config.repo_path),
        _ => todo!(),
    }
}
