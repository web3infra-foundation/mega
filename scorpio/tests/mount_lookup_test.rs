use scorpio::dicfuse::store::{DictionaryStore, PathLookupStatus};
use tempfile::tempdir;

#[tokio::test]
async fn lookup_status_found_normal_path() {
    let tmp = tempdir().unwrap();
    let store = DictionaryStore::new_with_store_path(tmp.path().to_str().unwrap()).await;

    store.insert_mock_item(1, 0, "", true).await;
    store.insert_mock_item(2, 1, "repo", true).await;
    store.insert_mock_item(3, 2, "a", true).await;

    let status = store.lookup_path_status("/repo/a").await.unwrap();
    assert_eq!(status, PathLookupStatus::Found(3));
}

#[tokio::test]
async fn lookup_status_temp_mount_add_temp_point() {
    let tmp = tempdir().unwrap();
    let store = DictionaryStore::new_with_store_path(tmp.path().to_str().unwrap()).await;

    store.insert_mock_item(1, 0, "", true).await;
    store.insert_mock_item(2, 1, "repo", true).await;

    let inode = store.add_temp_point("repo/tmp").await.unwrap();
    let status = store.lookup_path_status("/repo/tmp").await.unwrap();
    assert_eq!(status, PathLookupStatus::Found(inode));
}

#[tokio::test]
async fn lookup_status_not_found_normal_mount() {
    let tmp = tempdir().unwrap();
    let store = DictionaryStore::new_with_store_path(tmp.path().to_str().unwrap()).await;

    store.insert_mock_item(1, 0, "", true).await;
    store.insert_mock_item(2, 1, "repo", true).await;

    // Parent directory exists but has never been loaded; dicfuse must not claim NotFound.
    let status = store.lookup_path_status("/repo/missing").await.unwrap();
    assert_eq!(
        status,
        PathLookupStatus::ParentNotLoaded {
            parent_path: "/repo".to_string()
        }
    );
}

#[tokio::test]
async fn lookup_status_not_found_without_ancestor() {
    let tmp = tempdir().unwrap();
    let store = DictionaryStore::new_with_store_path(tmp.path().to_str().unwrap()).await;

    store.insert_mock_item(1, 0, "", true).await;

    let status = store.lookup_path_status("/missing").await.unwrap();
    assert_eq!(status, PathLookupStatus::NotFound);
}
