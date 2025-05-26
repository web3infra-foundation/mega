use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use ceres::protocol::mr::MergeRequest;
use common::model::{CommonPage, CommonResult, PageParams};

use crate::api::mr::{
    FilesChangedItem, FilesChangedList, MRDetail, MRStatusParams, MrInfoItem, SaveCommentRequest,
};
use crate::api::MonoApiServiceState;
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
            .routes(routes!(get_mr_files_changed))
            .routes(routes!(save_comment))
            .routes(routes!(delete_comment)),
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
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatusEnum::Closed {
            // util::check_permissions(
            //     &user.name,
            //     &model.path,
            //     ActionEnum::EditMergeRequest,
            //     state.clone(),
            // )
            // .await
            // .unwrap();
            let mut mr: MergeRequest = model.into();
            mr.status = MergeStatusEnum::Open;
            let res = match state.mr_stg().reopen_mr(mr.into(), 0, "admin").await {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
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
    // user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatusEnum::Open {
            // util::check_permissions(
            //     &user.name,
            //     &model.path,
            //     ActionEnum::EditMergeRequest,
            //     state.clone(),
            // )
            // .await
            // .unwrap();
            let mut mr: MergeRequest = model.into();
            mr.status = MergeStatusEnum::Closed;
            let res = match state.mr_stg().close_mr(mr.into(), 0, "admin").await {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
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
    // user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        if model.status == MergeStatusEnum::Open {
            // let path = model.path.clone();
            // util::check_permissions(
            //     &user.name,
            //     &path,
            //     ActionEnum::ApproveMergeRequest,
            //     state.clone(),
            // )
            // .await
            // .unwrap();
            let res = state.monorepo().merge_mr(&mut model.into()).await;
            let res = match res {
                Ok(_) => CommonResult::success(None),
                Err(err) => CommonResult::failed(&err.to_string()),
            };
            return Ok(Json(res));
        }
    }
    Ok(Json(CommonResult::failed("not found")))
}

/// Fetch MR list
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<MRStatusParams>,
    responses(
        (status = 200, body = CommonResult<CommonPage<MrInfoItem>>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn fetch_mr_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<MRStatusParams>>,
) -> Result<Json<CommonResult<CommonPage<MrInfoItem>>>, ApiError> {
    let status = json.additional.status;
    let status = if status == "open" {
        vec![MergeStatusEnum::Open]
    } else if status == "closed" {
        vec![MergeStatusEnum::Closed, MergeStatusEnum::Merged]
    } else {
        vec![
            MergeStatusEnum::Open,
            MergeStatusEnum::Closed,
            MergeStatusEnum::Merged,
        ]
    };
    let res = match state
        .mr_stg()
        .get_mr_by_status(status, json.pagination.page, json.pagination.per_page)
        .await
    {
        Ok((items, total)) => CommonResult::success(Some(CommonPage {
            items: items.into_iter().map(|m| m.into()).collect(),
            total,
        })),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

/// Get merge request details
#[utoipa::path(
    get,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/detail",
    responses(
        (status = 200, body = CommonResult<MRDetail>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn mr_detail(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<MRDetail>>, ApiError> {
    let res = match state.mr_stg().get_mr(&link).await {
        Ok(data) => {
            if let Some(model) = data {
                let mut detail: MRDetail = model.into();
                let conversations = state.mr_stg().get_mr_conversations(&link).await.unwrap();
                detail.conversations = conversations.into_iter().map(|x| x.into()).collect();
                CommonResult::success(Some(detail))
            } else {
                CommonResult::success(None)
            }
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
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
async fn get_mr_files_changed(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<FilesChangedList>>, ApiError> {
    let res = state.monorepo().content_diff(&link).await;
    let res = match res {
        Ok(data) => {
            let diff_files = extract_files_with_status(&data);
            let mut diff_list: Vec<FilesChangedItem> = vec![];
            for (path, status) in diff_files {
                diff_list.push(FilesChangedItem { path, status });
            }

            CommonResult::success(Some(FilesChangedList {
                files: diff_list,
                content: data,
            }))
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };

    Ok(Json(res))
}

/// Add new comment on Merge Request
#[utoipa::path(
    post,
    params(
        ("link", description = "MR link"),
    ),
    path = "/{link}/comment",
    request_body = SaveCommentRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn save_comment(
    // user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<SaveCommentRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = if let Some(model) = state.mr_stg().get_mr(&link).await.unwrap() {
        state
            .mr_stg()
            .add_mr_conversation(&model.link, 0, ConvTypeEnum::Comment, Some(payload.content))
            .await
            .unwrap();
        CommonResult::success(None)
    } else {
        CommonResult::failed("Invalid link")
    };
    Ok(Json(res))
}

/// Delete Comment
#[utoipa::path(
    delete,
    params(
        ("conv_id", description = "Conversation id"),
    ),
    path = "/comment/{conv_id}/delete",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = MR_TAG
)]
async fn delete_comment(
    Path(conv_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.mr_stg().remove_mr_conversation(conv_id).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::api::mr::mr_router::extract_files_with_status;

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
            println!("{} ({})", file, status);
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
}
