use std::collections::HashSet;

use api_model::common::CommonResult;
use axum::{Json, extract::State};
use callisto::sea_orm_active_enums::{ConvTypeEnum, ReferenceTypeEnum};
use regex::Regex;

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};

pub fn parse_data_id(comment: &str) -> HashSet<String> {
    let data_id = Regex::new(r#"data-id="([A-Za-z0-9]+)""#).unwrap();
    let links: HashSet<String> = data_id
        .captures_iter(comment)
        .map(|cap| cap[1].to_string())
        .collect();
    links
}

pub async fn check_comment_ref(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    comment: &str,
    source_link: &str,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let links = parse_data_id(comment);
    let username = user.username;
    for ref_link in links {
        state
            .issue_stg()
            .add_reference(source_link, &ref_link, ReferenceTypeEnum::Mention)
            .await?;
        state
            .conv_stg()
            .add_conversation(
                &ref_link,
                &username,
                Some(format!("{username} mentioned this on")),
                ConvTypeEnum::Mention,
            )
            .await?;
    }

    Ok(Json(CommonResult::success(None)))
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::api::api_common::comment::parse_data_id;

    #[test]
    pub fn test_parse_data_id_from_comment() {
        let single_ref = r#"<p><span class="link-issue" data-type="linkIssue" data-id="HDQL6ATY" data-label="HDQL6ATY" data-suggestiontype="change_list">$HDQL6ATY</span> </p>"#;
        let data_id = parse_data_id(single_ref);
        assert_eq!(
            data_id.iter().next(),
            Some(String::from("HDQL6ATY")).as_ref()
        );

        let normal_comment = r#"<p>This is a normal comment without reference</p>"#;
        let data_id = parse_data_id(normal_comment);
        assert_eq!(data_id.iter().next(), None);

        let multi_referene = r#"<p>multireference?? <span class="link-issue" data-type="linkIssue" data-id="PIXLXMS9" data-label="PIXLXMS9" data-suggestiontype="issue">$PIXLXMS9</span> <span class="link-issue" data-type="linkIssue" data-id="PIXLXMS9" data-label="PIXLXMS9" data-suggestiontype="issue">$PIXLXMS9</span> <span class="link-issue" data-type="linkIssue" data-id="ZE710J7U" data-label="ZE710J7U" data-suggestiontype="change_list">$ZE710J7U</span> </p>"#;
        let data_id = parse_data_id(multi_referene);
        assert_eq!(
            data_id,
            HashSet::from([String::from("PIXLXMS9"), String::from("ZE710J7U")])
        );
    }
}
