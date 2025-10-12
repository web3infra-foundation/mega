use std::path::PathBuf;

use axum::{
    Json,
    extract::{Path, State},
};
use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use common::{
    errors::MegaError,
    model::{CommonPage, CommonResult, PageParams},
};
use jupiter::service::cl_service::CLService;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::cl::model::{
    ChangeReviewStatePayload, ChangeReviewerStatePayload, CloneRepoPayload, ReviewerInfo,
    ReviewerPayload, ReviewersResponse,
};
use crate::api::{MonoApiServiceState, cl::FilesChangedPage};
use crate::api::{
    api_common::{
        self,
        model::{AssigneeUpdatePayload, ListPayload},
    },
    cl::{CLDetailRes, ClFilesRes, Condition, MergeBoxRes, MuiTreeNode},
    conversation::ContentPayload,
    issue::ItemRes,
    label::LabelUpdatePayload,
    oauth::model::LoginUser,
};
use crate::{api::error::ApiError, server::http_server::MR_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/mr",
        OpenApiRouter::new()
            .routes(routes!(fetch_cl_list))
            .routes(routes!(cl_detail))
            .routes(routes!(merge))
            .routes(routes!(merge_box))
            .routes(routes!(merge_no_auth))
            .routes(routes!(close_cl))
            .routes(routes!(reopen_cl))
            .routes(routes!(cl_mui_tree))
            .routes(routes!(cl_files_changed_by_page))
            .routes(routes!(cl_files_list))
            .routes(routes!(save_comment))
            .routes(routes!(labels))
            .routes(routes!(assignees))
            .routes(routes!(edit_title))
            .routes(routes!(add_reviewers))
            .routes(routes!(remove_reviewers))
            .routes(routes!(list_reviewers))
            .routes(routes!(reviewer_approve))
            .routes(routes!(review_resolve))
            .routes(routes!(clone_third_party_repo)),
    )
}

/// Reopen Change List
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/reopen",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn reopen_cl(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.cl_stg().get_cl(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Closed {
        // util::check_permissions(
        //     &user.name,
        //     &model.path,
        //     ActionEnum::EditChangeList,
        //     state.clone(),
        // )
        // .await
        // .unwrap();

        let link = model.link.clone();
        state.cl_stg().reopen_cl(model).await?;
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

/// Close Change List
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/close",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn close_cl(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.cl_stg().get_cl(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Open {
        // util::check_permissions(
        //     &user.name,
        //     &model.path,
        //     ActionEnum::EditChangeList,
        //     state.clone(),
        // )
        // .await
        // .unwrap();
        let link = model.link.clone();
        state.cl_stg().close_cl(model).await?;
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

/// Approve Change List
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
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
    let res = state.cl_stg().get_cl(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;

    if model.status == MergeStatusEnum::Open {
        // let path = model.path.clone();
        // util::check_permissions(
        //     &user.username,
        //     &path,
        //     ActionEnum::ApproveChangeList,
        //     state.clone(),
        // )
        // .await
        // .unwrap();
        state.monorepo().merge_cl(&user.username, model).await?;
    }
    Ok(Json(CommonResult::success(None)))
}

/// Change List without authentication
/// It's for local testing purposes.
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
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
    let res = state.cl_stg().get_cl(&link).await?;
    let model = res.ok_or(MegaError::with_message("CL Not Found"))?;

    if model.status != MergeStatusEnum::Open {
        return Err(ApiError::from(MegaError::with_message(format!(
            "CL is not in Open status, current status: {:?}",
            model.status
        ))));
    }

    // No authentication required - using default system user
    let default_username = "system";
    state.monorepo().merge_cl(default_username, model).await?;

    Ok(Json(CommonResult::success(Some(
        "Merge completed successfully".to_string(),
    ))))
}

/// Fetch CL list
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<ListPayload>,
    responses(
        (status = 200, body = CommonResult<CommonPage<ItemRes>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn fetch_cl_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<ListPayload>>,
) -> Result<Json<CommonResult<CommonPage<ItemRes>>>, ApiError> {
    let (items, total) = state
        .cl_stg()
        .get_cl_list(json.additional.into(), json.pagination)
        .await?;
    let res = CommonPage {
        items: items.into_iter().map(|m| m.into()).collect(),
        total,
    };
    Ok(Json(CommonResult::success(Some(res))))
}

/// Get change list details
#[utoipa::path(
    get,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/detail",
    responses(
        (status = 200, body = CommonResult<CLDetailRes>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn cl_detail(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<CLDetailRes>>, ApiError> {
    let cl_service: CLService = state.storage.cl_service.clone();
    let cl_details: CLDetailRes = cl_service
        .get_cl_details(&link, user.username)
        .await?
        .into();
    Ok(Json(CommonResult::success(Some(cl_details))))
}

#[utoipa::path(
    get,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/mui-tree",
    responses(
        (status = 200, body = CommonResult<Vec<MuiTreeNode>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn cl_mui_tree(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<MuiTreeNode>>>, ApiError> {
    let files = state
        .monorepo()
        .get_sorted_changed_file_list(&link, None)
        .await?;
    let mui_trees = build_forest(files);
    Ok(Json(CommonResult::success(Some(mui_trees))))
}

/// Get Change List file changed list in Pagination
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/files-changed",
    request_body = PageParams<String>,
    responses(
        (status = 200, body = CommonResult<FilesChangedPage>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn cl_files_changed_by_page(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<String>>,
) -> Result<Json<CommonResult<FilesChangedPage>>, ApiError> {
    let (items, total) = state
        .monorepo()
        .paged_content_diff(&link, json.pagination)
        .await?;
    let res = CommonResult::success(Some(FilesChangedPage {
        page: CommonPage { total, items },
    }));
    Ok(Json(res))
}

/// Get Change List file list
#[utoipa::path(
    get,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/files-list",
    responses(
        (status = 200, body = CommonResult<Vec<ClFilesRes>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn cl_files_list(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ClFilesRes>>>, ApiError> {
    let cl = state
        .cl_stg()
        .get_cl(&link)
        .await?
        .ok_or(MegaError::with_message("CL Not Found"))?;

    let stg = state.monorepo();
    let old_files = stg.get_commit_blobs(&cl.from_hash).await?;
    let new_files = stg.get_commit_blobs(&cl.to_hash).await?;
    let cl_diff_files = stg.cl_files_list(old_files, new_files.clone()).await?; // TODO

    let cl_base = PathBuf::from(cl.path);
    let res = cl_diff_files
        .into_iter()
        .map(|m| {
            let mut item: ClFilesRes = m.into();
            item.path = cl_base.join(item.path).to_string_lossy().to_string();
            item
        })
        .collect::<Vec<ClFilesRes>>();
    Ok(Json(CommonResult::success(Some(res))))
}

/// Get Merge Box to check merge status
#[utoipa::path(
    get,
    params(
        ("link", description = "CL link"),
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
    let cl = state
        .cl_stg()
        .get_cl(&link)
        .await?
        .ok_or(MegaError::with_message("CL Not Found"))?;

    let res = match cl.status {
        MergeStatusEnum::Open => {
            let check_res: Vec<Condition> = state
                .cl_stg()
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

/// Add new comment on Change List
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
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
    let conv_type = if state
        .storage
        .reviewer_storage()
        .is_reviewer(&link, &user.username)
        .await?
    {
        // If user is the reviewer for this cl, then the comment if of type review
        ConvTypeEnum::Review
    } else {
        ConvTypeEnum::Comment
    };

    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(payload.content.clone()),
            conv_type,
        )
        .await?;
    api_common::comment::check_comment_ref(user, state, &payload.content, &link).await
}

/// Edit CL title
#[utoipa::path(
    post,
    params(
        ("link", description = "A string ID representing a Change List"),
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
    state.cl_stg().edit_title(&link, &payload.content).await?;
    Ok(Json(CommonResult::success(None)))
}

/// Update cl related labels
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
    api_common::label_assignee::label_update(user, state, payload, String::from("cl")).await
}

/// Update CL related assignees
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
    api_common::label_assignee::assignees_update(user, state, payload, String::from("cl")).await
}

#[utoipa::path(
    post,
    params (
        ("link", description = "the cl link")
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
    state
        .storage
        .reviewer_storage()
        .add_reviewers(&link, payload.reviewer_usernames)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    delete,
    params (
        ("link", description = "the cl link"),
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
    state
        .storage
        .reviewer_storage()
        .remove_reviewers(&link, payload.reviewer_usernames)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    get,
    params (
        ("link", description = "the cl link")
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
    let reviewers = state
        .storage
        .reviewer_storage()
        .list_reviewers(&link)
        .await?
        .into_iter()
        .map(|r| ReviewerInfo {
            username: r.username,
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
        ("link", description = "the cl link")
    ),
    path = "/{link}/reviewer/approve",
    request_body = ChangeReviewerStatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn reviewer_approve(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ChangeReviewerStatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    state
        .storage
        .reviewer_storage()
        .reviewer_change_state(&link, &user.username, payload.approved)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    post,
    params (
        ("link", description = "the cl link")
    ),
    path = "/{link}/review/resolve",
    request_body (
        content = ChangeReviewStatePayload,
    ),
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn review_resolve(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(link): Path<String>,
    Json(payload): Json<ChangeReviewStatePayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .storage
        .cl_storage()
        .is_assignee(&link, &user.username)
        .await?;

    state
        .storage
        .conversation_storage()
        .change_review_state(&link, &payload.conversation_id, payload.resolved)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

// Clone a Github Repo
#[utoipa::path(
    post,
    path = "/clone",
    request_body (
        content = CloneRepoPayload,
    ),
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn clone_third_party_repo(
    state: State<MonoApiServiceState>,
    Json(payload): Json<CloneRepoPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .monorepo()
        .sync_third_party_repo(&payload.owner, &payload.repo)
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
    use crate::api::cl::cl_router::build_forest;
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
    fn test_cl_files_changed_logic() {
        // Test the core logic of cl_files_changed function
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
