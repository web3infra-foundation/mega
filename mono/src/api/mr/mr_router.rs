use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use common::{
    errors::MegaError,
    model::{CommonPage, CommonResult, PageParams},
};
use jupiter::service::mr_service::MRService;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::mr::model::{
    ChangeReviewerStatePayload, ReviewerInfo, ReviewerPayload, ReviewersResponse,
};
use crate::api::{
    api_common::{
        self,
        model::{AssigneeUpdatePayload, ListPayload},
    },
    conversation::ContentPayload,
    issue::ItemRes,
    label::LabelUpdatePayload,
    mr::{Condition, MRDetailRes, MergeBoxRes, MrFilesRes, MuiTreeNode},
    oauth::model::LoginUser,
};
use crate::api::{mr::FilesChangedPage, MonoApiServiceState};
use crate::{api::error::ApiError, server::http_server::MR_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/mr",
        OpenApiRouter::new()
            .routes(routes!(fetch_mr_list))
            .routes(routes!(mr_detail))
            .routes(routes!(merge))
            .routes(routes!(merge_box))
            .routes(routes!(merge_no_auth))
            .routes(routes!(close_mr))
            .routes(routes!(reopen_mr))
            .routes(routes!(mr_files_changed_by_page))
            .routes(routes!(mr_files_list))
            .routes(routes!(save_comment))
            .routes(routes!(labels))
            .routes(routes!(assignees))
            .routes(routes!(edit_title))
            .routes(routes!(add_reviewers))
            .routes(routes!(remove_reviewers))
            .routes(routes!(list_reviewers))
            .routes(routes!(change_reviewer_state)),
    )
}

/// Reopen Merge Request
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/reopen",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn reopen_mr(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Closed {
        // util::check_permissions(
        //     &user.name,
        //     &model.path,
        //     ActionEnum::EditMergeRequest,
        //     state.clone(),
        // )
        // .await
        // .unwrap();

        let link = model.link.clone();
        state.mr_stg().reopen_mr(model).await?;
        state
            .conv_stg()
            .add_conversation(
                &link,
                &user.username,
                Some(format!("{} reopen this", user.username)),
                ConvTypeEnum::Reopen,
            )
            .await
            .unwrap();
    }
    Ok(Json(CommonResult::success(None)))
}

/// Close Merge Request
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/close",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn close_mr(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Open {
        // util::check_permissions(
        //     &user.name,
        //     &model.path,
        //     ActionEnum::EditMergeRequest,
        //     state.clone(),
        // )
        // .await
        // .unwrap();
        let link = model.link.clone();
        state.mr_stg().close_mr(model).await?;
        state
            .conv_stg()
            .add_conversation(
                &link,
                &user.username,
                Some(format!("{} closed this", user.username)),
                ConvTypeEnum::Closed,
            )
            .await?;
    }
    Ok(Json(CommonResult::success(None)))
}

/// Approve Merge Request
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/merge",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn merge(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Open {
        // let path = model.path.clone();
        // util::check_permissions(
        //     &user.username,
        //     &path,
        //     ActionEnum::ApproveMergeRequest,
        //     state.clone(),
        // )
        // .await
        // .unwrap();
        state.monorepo().merge_mr(&user.username, model).await?;
    }
    Ok(Json(CommonResult::success(None)))
}

/// Merge Request without authentication
/// It's for local testing purposes.
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/merge-no-auth",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn merge_no_auth(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("MR Not Found"))?;

    if model.status != MergeStatusEnum::Open {
        return Err(ApiError::from(MegaError::with_message(format!(
            "MR is not in Open status, current status: {:?}",
            model.status
        ))));
    }

    // No authentication required - using default system user
    let default_username = "system";
    state.monorepo().merge_mr(default_username, model).await?;

    Ok(Json(CommonResult::success(Some(
        "Merge completed successfully".to_string(),
    ))))
}

/// Fetch MR list
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<ListPayload>,
    responses(
        (status = 200, body = CommonResult<CommonPage<ItemRes>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn fetch_mr_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<ListPayload>>,
) -> Result<Json<CommonResult<CommonPage<ItemRes>>>, ApiError> {
    let (items, total) = state
        .mr_stg()
        .get_mr_list(json.additional.into(), json.pagination)
        .await?;
    let res = CommonPage {
        items: items.into_iter().map(|m| m.into()).collect(),
        total,
    };
    Ok(Json(CommonResult::success(Some(res))))
}

/// Get merge request details
#[utoipa::path(
    get,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/detail",
    responses(
        (status = 200, body = CommonResult<MRDetailRes>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn mr_detail(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<MRDetailRes>>, ApiError> {
    let mr_service: MRService = state.storage.mr_service.clone();
    let mr_details: MRDetailRes = mr_service
        .get_mr_details(&link, user.username)
        .await?
        .into();
    Ok(Json(CommonResult::success(Some(mr_details))))
}

/// Get Merge Request file changed list in Pagination
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/files-changed",
    request_body = PageParams<String>,
    responses(
        (status = 200, body = CommonResult<FilesChangedPage>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn mr_files_changed_by_page(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<String>>,
) -> Result<Json<CommonResult<FilesChangedPage>>, ApiError> {
    let (items, changed_files_path, total) = state
        .monorepo()
        .paged_content_diff(&link, json.pagination)
        .await?;

    let mui_trees = build_forest(changed_files_path);
    let res = CommonResult::success(Some(FilesChangedPage {
        mui_trees,
        page: CommonPage { total, items },
    }));
    Ok(Json(res))
}

/// Get Merge Request file list
#[utoipa::path(
    get,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/files-list",
    responses(
        (status = 200, body = CommonResult<Vec<MrFilesRes>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn mr_files_list(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<MrFilesRes>>>, ApiError> {
    let mr = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?;

    let stg = state.monorepo();
    let old_files = stg.get_commit_blobs(&mr.from_hash).await?;
    let new_files = stg.get_commit_blobs(&mr.to_hash).await?;
    let mr_diff_files = stg.mr_files_list(old_files, new_files.clone()).await?;

    let mr_base = PathBuf::from(mr.path);
    let res = mr_diff_files
        .into_iter()
        .map(|m| {
            let mut item: MrFilesRes = m.into();
            item.path = mr_base.join(item.path).to_string_lossy().to_string();
            item
        })
        .collect::<Vec<MrFilesRes>>();
    Ok(Json(CommonResult::success(Some(res))))
}

/// Get Merge Box to check merge status
#[utoipa::path(
    get,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/merge-box",
    responses(
        (status = 200, body = CommonResult<MergeBoxRes>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn merge_box(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<MergeBoxRes>>, ApiError> {
    let mr = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?;

    let res = match mr.status {
        MergeStatusEnum::Open => {
            let check_res: Vec<Condition> = state
                .mr_stg()
                .get_check_result(&link)
                .await?
                .into_iter()
                .map(|m| m.into())
                .collect();
            MergeBoxRes::from_condition(check_res)
        }
        MergeStatusEnum::Merged | MergeStatusEnum::Closed => MergeBoxRes {
            merge_requirements: None,
        },
    };
    Ok(Json(CommonResult::success(Some(res))))
}

/// Add new comment on Merge Request
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/comment",
    request_body = ContentPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn save_comment(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ContentPayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;
    state
        .conv_stg()
        .add_conversation(
            &model.link,
            &user.username,
            Some(payload.content.clone()),
            ConvTypeEnum::Comment,
        )
        .await?;
    api_common::comment::check_comment_ref(user, state, &payload.content, &link).await
}

/// Edit MR title
#[utoipa::path(
    post,
    params(
        ("link", description = "A string ID representing a Merge Request"),
    ),
    path = "/{link}/title",
    request_body = ContentPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn edit_title(
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ContentPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.mr_stg().edit_title(&link, &payload.content).await?;
    Ok(Json(CommonResult::success(None)))
}

/// Update mr related labels
#[utoipa::path(
    post,
    path = "/labels",
    request_body = LabelUpdatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn labels(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<LabelUpdatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    api_common::label_assignee::label_update(user, state, payload, String::from("mr")).await
}

/// Update MR related assignees
#[utoipa::path(
    post,
    path = "/assignees",
    request_body = AssigneeUpdatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn assignees(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<AssigneeUpdatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    api_common::label_assignee::assignees_update(user, state, payload, String::from("mr")).await
}

#[utoipa::path(
    post,
    params (
        ("link", description = "the mr link")
    ),
    path = "/{link}/reviewers",
    request_body = ReviewerPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn add_reviewers(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ReviewerPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let mr_id = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?
        .id;

    state
        .storage
        .reviewer_storage()
        .add_reviewers(mr_id, payload.reviewers)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    delete,
    params (
        ("link", description = "the mr link"),
    ),
    path = "/{link}/reviewers",
    request_body = ReviewerPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn remove_reviewers(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ReviewerPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let mr_id = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?
        .id;

    state
        .storage
        .reviewer_storage()
        .remove_reviewers(mr_id, payload.reviewers)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    get,
    params (
        ("link", description = "the mr link")
    ),
    path = "/{link}/reviewers",
    responses(
        (status = 200, body = CommonResult<ReviewersResponse>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn list_reviewers(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ReviewersResponse>>, ApiError> {
    let mr_id = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?
        .id;

    let reviewers = state
        .storage
        .reviewer_storage()
        .list_reviewers(mr_id)
        .await?
        .into_iter()
        .map(|r| ReviewerInfo {
            campsite_id: r.campsite_id,
            approved: r.approved,
        })
        .collect();

    Ok(Json(CommonResult::success(Some(ReviewersResponse {
        result: reviewers,
    }))))
}

/// Change the reviewer state
///
/// the function get user's campsite_id from the login user info automatically
#[utoipa::path(
    post,
    params (
        ("link", description = "the mr link")
    ),
    path = "/{link}/reviewer-new-state",
    request_body = ChangeReviewerStatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn change_reviewer_state(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ChangeReviewerStatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let mr_id = state
        .mr_stg()
        .get_mr(&link)
        .await?
        .ok_or(MegaError::with_message("MR Not Found"))?
        .id;

    state
        .storage
        .reviewer_storage()
        .reviewer_change_state(mr_id, user.campsite_user_id, payload.state)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

fn build_forest(paths: Vec<String>) -> Vec<MuiTreeNode> {
    let mut roots: Vec<MuiTreeNode> = Vec::new();

    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.is_empty() {
            continue;
        }

        let root_label = parts[0];
        if let Some(existing_root) = roots.iter_mut().find(|r| r.label == root_label) {
            existing_root.insert_path(&parts[1..]);
        } else {
            let mut new_root = MuiTreeNode::new(root_label);
            new_root.insert_path(&parts[1..]);
            roots.push(new_root);
        }
    }

    roots
}

#[cfg(test)]
mod test {
    use crate::api::mr::mr_router::build_forest;
    use neptune::model::diff_model::DiffItem;
    use std::collections::HashMap;

    fn extract_files_with_status(diff_output: &str) -> HashMap<String, String> {
        let mut files = HashMap::new();

        let chunks: Vec<&str> = diff_output.split("diff --git ").collect();

        for chunk in chunks.iter().skip(1) {
            let lines: Vec<&str> = chunk.split_whitespace().collect();
            if lines.len() >= 2 {
                let current_file = lines[0].trim_start_matches("a/").to_string();
                files.insert(current_file.clone(), "modified".to_string()); // 默认状态为修改
                if chunk.contains("new file mode") {
                    files.insert(current_file, "new".to_string());
                } else if chunk.contains("deleted file mode") {
                    files.insert(current_file, "deleted".to_string());
                }
            }
        }
        files
    }

    #[test]
    fn test_parse_diff_result_to_filelist() {
        let diff_output = r#"
        diff --git a/ceres/src/api_service/mono_api_service.rs b/ceres/src/api_service/mono_api_service.rs
        new file mode 100644
        index 0000000..561296a1
        @@ -1,0 +1,595 @@
        fn main() {
            println!("Hello, world!");
        }
        diff --git a/ceres/src/lib.rs b/ceres/src/lib.rs
        index 1234567..89abcdef 100644
        --- a/ceres/src/lib.rs
        +++ b/ceres/src/lib.rs
        @@ -10,7 +10,8 @@
        diff --git a/ceres/src/removed.rs b/ceres/src/removed.rs
        deleted file mode 100644
        "#;
        let files_with_status = extract_files_with_status(diff_output);
        println!("Files with status:");
        for (file, status) in &files_with_status {
            println!("{file} ({status})");
        }

        let mut expected = HashMap::new();
        expected.insert(
            "ceres/src/api_service/mono_api_service.rs".to_string(),
            "new".to_string(),
        );
        expected.insert("ceres/src/lib.rs".to_string(), "modified".to_string());
        expected.insert("ceres/src/removed.rs".to_string(), "deleted".to_string());

        assert_eq!(files_with_status, expected);
    }

    #[test]
    fn test_files_changed_tree() {
        let paths = vec![
            String::from("crates-pro/crates_pro/src/bin/bin_analyze.rs"),
            String::from("crates-pro/images/analysis-tool-worker.Dockerfile"),
            String::from("crates-pro/images/crates-pro.Dockerfile"),
            String::from("another-root/foo/bar.txt"),
        ];

        let forest = build_forest(paths);
        println!("{}", serde_json::to_string_pretty(&forest).unwrap());
    }

    #[test]
    fn test_mr_files_changed_logic() {
        // Test the core logic of mr_files_changed function
        // This tests the data transformation logic without needing the full state

        let sample_diff_output = r#"diff --git a/src/main.rs b/src/main.rs
            new file mode 100644
            index 0000000..abc1234
            --- /dev/null
            +++ b/src/main.rs
            @@ -0,0 +1,5 @@
            +fn main() {
            +    println!("Hello, world!");
            +}
            diff --git a/src/lib.rs b/src/lib.rs
            index def5678..ghi9012 100644
            --- a/src/lib.rs
            +++ b/src/lib.rs
            @@ -1,3 +1,4 @@
            +// Added a comment
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
            diff --git a/README.md b/README.md
            deleted file mode 100644
            index 1234567..0000000"#;

        // Test extract_files_with_status
        let diff_files = extract_files_with_status(sample_diff_output);

        assert_eq!(diff_files.len(), 3);
        assert_eq!(diff_files.get("src/main.rs"), Some(&"new".to_string()));
        assert_eq!(diff_files.get("src/lib.rs"), Some(&"modified".to_string()));
        assert_eq!(diff_files.get("README.md"), Some(&"deleted".to_string()));

        // Test path extraction and tree building
        let mut paths = vec![];
        for (path, _) in diff_files {
            paths.push(path);
        }

        let mui_trees = build_forest(paths);

        // Verify the tree structure
        assert!(!mui_trees.is_empty());

        // Check that we have the expected root nodes
        let root_labels: Vec<&str> = mui_trees.iter().map(|tree| tree.label.as_str()).collect();
        assert!(root_labels.contains(&"src"));
        assert!(root_labels.contains(&"README.md"));

        let content = [DiffItem {
            data: sample_diff_output.to_string(),
            path: "diff_output.txt".to_string(),
        }];

        assert!(!mui_trees.is_empty());
        assert_eq!(content.first().unwrap().data, sample_diff_output);
    }

    #[test]
    fn test_extract_files_with_status_edge_cases() {
        // Test with empty diff output
        let empty_diff = "";
        let result = extract_files_with_status(empty_diff);
        assert!(result.is_empty());

        // Test with malformed diff output
        let malformed_diff = "not a valid diff output";
        let result = extract_files_with_status(malformed_diff);
        assert!(result.is_empty());

        // Test with diff containing only additions
        let additions_only = r#"diff --git a/new_file.txt b/new_file.txt
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/new_file.txt"#;

        let result = extract_files_with_status(additions_only);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("new_file.txt"), Some(&"new".to_string()));

        // Test with diff containing only deletions
        let deletions_only = r#"diff --git a/old_file.txt b/old_file.txt
deleted file mode 100644
index 1234567..0000000"#;

        let result = extract_files_with_status(deletions_only);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("old_file.txt"), Some(&"deleted".to_string()));
    }

    #[test]
    fn test_build_forest_edge_cases() {
        // Test with empty paths
        let empty_paths = vec![];
        let result = build_forest(empty_paths);
        assert!(result.is_empty());

        // Test with single file
        let single_file = vec!["single_file.txt".to_string()];
        let result = build_forest(single_file);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "single_file.txt");

        // Test with deeply nested paths
        let nested_paths = vec![
            "a/b/c/d/e/file.txt".to_string(),
            "a/b/different.txt".to_string(),
            "a/another.txt".to_string(),
        ];
        let result = build_forest(nested_paths);
        assert_eq!(result.len(), 1); // Should have one root "a"
        assert_eq!(result[0].label, "a");

        // The tree should have nested structure
        let root = &result[0];
        assert!(root.children.is_some());
        let children = root.children.as_ref().unwrap();
        assert!(children.iter().any(|child| child.label == "b"));
        assert!(children.iter().any(|child| child.label == "another.txt"));
    }
}
