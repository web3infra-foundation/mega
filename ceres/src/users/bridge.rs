use std::sync::Arc;
use std::time::Duration;

use common::config::Config;
use http::header::COOKIE;
use reqwest::Client;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RemoteUser {
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
}

/// Fetch organization member info by username via `/v1/organizations/{org_slug}/members/{username}`
pub async fn get_org_member_by_username(
    config: Arc<Config>,
    org_slug: &str,
    username: &str,
    cookie_header: Option<String>,
) -> anyhow::Result<Option<RemoteUser>> {
    let base = config
        .oauth
        .as_ref()
        .map(|o| o.campsite_api_domain.clone())
        .unwrap_or_else(|| std::env::var("MEGA_USERS_API_BASE").unwrap_or_default());
    let token = std::env::var("MEGA_USERS_API_TOKEN").ok();
    if base.is_empty() {
        return Ok(None);
    }

    let url = format!(
        "{}/v1/organizations/{}/members/{}",
        base, org_slug, username
    );
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let mut req = client.get(url);
    if let Some(tok) = token {
        req = req.bearer_auth(tok);
    }
    if let Some(cookie) = cookie_header.as_ref() {
        req = req.header(COOKIE, cookie);
    }
    let resp = req.send().await?;
    let code = resp.status();
    debug!(
        target: "users_bridge",
        org_slug = org_slug,
        username = username,
        status = %code,
        "org-member lookup finished"
    );
    if code.as_u16() == 404 {
        return Ok(None);
    }
    if !code.is_success() {
        warn!(
            target = "users_bridge",
            org_slug = org_slug,
            username = username,
            status = %code,
            "org-member lookup not successful"
        );
        return Ok(None);
    }

    let obj: serde_json::Value = resp.json().await?;
    // Campsite API returns an OrganizationMember with nested `user` object.
    // Also be defensive for alternative shapes like `{ member: { user: {...} } }`.
    let user_obj = obj
        .get("user")
        .or_else(|| obj.get("member").and_then(|m| m.get("user")))
        .unwrap_or(&obj);

    let returned_username = user_obj
        .get("username")
        .or_else(|| user_obj.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if returned_username.is_empty() {
        return Ok(None);
    }

    let display_name = user_obj
        .get("display_name")
        .or_else(|| user_obj.get("displayName"))
        .or_else(|| user_obj.get("username"))
        .or_else(|| user_obj.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let avatar_url = user_obj
        .get("avatar_url")
        .or_else(|| user_obj.get("avatarUrl"))
        .or_else(|| {
            user_obj
                .get("avatar_urls")
                .and_then(|a| a.get("thumbnail_url"))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Some(RemoteUser {
        username: returned_username,
        display_name,
        avatar_url,
    }))
}
