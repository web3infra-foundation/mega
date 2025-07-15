use std::{collections::HashMap, path::PathBuf};

use axum::{
    extract::{Path, State},
    Json,
};
use jupiter::service::mr_service::MRService;
use utoipa_axum::{router::OpenApiRouter, routes};

use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use common::{
    errors::MegaError,
    model::{CommonPage, CommonResult, PageParams},
};
use saturn::ActionEnum;

use crate::api::{
    api_common::{
        self,
        model::{AssigneeUpdatePayload, ListPayload},
    },
    conversation::ContentPayload,
    issue::ItemRes,
    label::LabelUpdatePayload,
    mr::{FilesChangedList, MRDetailRes, MrFilesRes, MuiTreeNode},
    oauth::model::LoginUser,
};
use crate::api::{util, MonoApiServiceState};
use crate::{api::error::ApiError, server::https_server::MR_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/mr",
        OpenApiRouter::new()
            .routes(routes!(fetch_mr_list))
            .routes(routes!(mr_detail))
            .routes(routes!(merge))
            .routes(routes!(close_mr))
            .routes(routes!(reopen_mr))
            .routes(routes!(mr_files_changed))
            .routes(routes!(mr_files_list))
            .routes(routes!(save_comment))
            .routes(routes!(labels))
            .routes(routes!(assignees))
            .routes(routes!(edit_title)),
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
        let path = model.path.clone();
        util::check_permissions(
            &user.username,
            &path,
            ActionEnum::ApproveMergeRequest,
            state.clone(),
        )
        .await
        .unwrap();
        state.monorepo().merge_mr(&user.username, model).await?;
    }
    Ok(Json(CommonResult::success(None)))
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

/// Get Merge Request file changed list
#[utoipa::path(
    get,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/files-changed",
    responses(
        (status = 200, body = CommonResult<FilesChangedList>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn mr_files_changed(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<FilesChangedList>>, ApiError> {
    let listen_addr = &state.listen_addr;
    let diff_res = state.monorepo().content_diff(&link, listen_addr).await?;

    let diff_files = extract_files_with_status(&diff_res);
    let mut paths = vec![];
    for (path, _) in diff_files {
        paths.push(path);
    }
    let mui_trees = build_forest(paths);
    let res = CommonResult::success(Some(FilesChangedList {
        mui_trees,
        content: diff_res,
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
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.mr_stg().get_mr(&link).await?;
    let model = res.ok_or(MegaError::with_message("Not Found"))?;
    state
        .conv_stg()
        .add_conversation(
            &model.link,
            &user.username,
            Some(payload.content),
            ConvTypeEnum::Comment,
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
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
    state
        .mr_stg()
        .edit_title(&link, &payload.content)
        .await?;
    Ok(Json(CommonResult::success(None)))
}

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
    use std::collections::HashMap;

    use crate::api::mr::mr_router::{build_forest, extract_files_with_status};

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
}
