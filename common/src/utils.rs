use idgenerator::IdInstance;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;
use serde_json::{json, Value};

pub const ZERO_ID: &str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

pub fn generate_id() -> i64 {
    // Call `next_id` to generate a new unique id.
    IdInstance::next_id()
}

pub fn generate_link() -> String {
    let str: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    str.to_uppercase()
}

pub const MEGA_BRANCH_NAME: &str = "refs/heads/main";

pub fn generate_rich_text(content: &str) -> String {
    let json_str = r#"
    {
        "root": {
            "children": [{
                "children": [{ "detail": 0, "format": 0, "mode": "normal", "style": "", "text": "", "type": "text", "version": 1 }],
                "direction": "ltr", "format": "", "indent": 0, "type": "paragraph", "version": 1, "textFormat": 0, "textStyle": ""
            }], "direction": "ltr", "format": "", "indent": 0, "type": "root", "version": 1
        }
    }"#;
    let mut data: Value = serde_json::from_str(json_str).expect("Invalid JSON");

    if let Some(text_value) = data["root"]["children"][0]["children"][0].get_mut("text") {
        *text_value = json!(content);
    }
    serde_json::to_string_pretty(&data).expect("Failed to serialize JSON")
}

pub fn mr_ref_name(mr_link: &str) -> String {
    format!("refs/heads/{}", mr_link)
}

/// Format commit message with GPG signature<br>
/// There must be a `blank line`(\n) before `message`, or remote unpack failed.<br>
/// If there is `GPG signature`,
/// `blank line` should be placed between `signature` and `message`
pub fn format_commit_msg(msg: &str, gpg_sig: Option<&str>) -> String {
    match gpg_sig {
        None => {
            format!("\n{}", msg)
        }
        Some(gpg) => {
            format!("{}\n\n{}", gpg, msg)
        }
    }
}

/// parse commit message
pub fn parse_commit_msg(msg_gpg: &str) -> (String, Option<String>) {
    const GPG_SIG_START: &str = "gpgsig -----BEGIN PGP SIGNATURE-----";
    const GPG_SIG_END: &str = "-----END PGP SIGNATURE-----";
    let gpg_start = msg_gpg.find(GPG_SIG_START);
    let gpg_end = msg_gpg.find(GPG_SIG_END).map(|end| end + GPG_SIG_END.len());
    let gpg_sig = match (gpg_start, gpg_end) {
        (Some(start), Some(end)) => {
            if start < end {
                Some(msg_gpg[start..end].to_string())
            } else {
                None
            }
        }
        _ => None,
    };
    match gpg_sig {
        Some(gpg) => {
            // Skip the leading '\n\n' (blank lines).
            // Some commit messages may use '\n \n\n' or similar patterns.
            // To handle such cases, remove all leading blank lines from the message.
            let msg = msg_gpg[gpg_end.unwrap()..].trim_start().to_string();
            (msg, Some(gpg))
        }
        None => {
            assert!(msg_gpg.starts_with('\n'), "commit message format error");
            let msg = msg_gpg[1..].to_string(); // skip the leading '\n' (blank line)
            (msg, None)
        }
    }
}

// check if the commit message is conventional commit
// ref: https://www.conventionalcommits.org/en/v1.0.0/
pub fn check_conventional_commits_message(msg: &str) -> bool {
    let first_line = msg.lines().next().unwrap_or_default();
    #[allow(unused_variables)]
    let body_footer = msg.lines().skip(1).collect::<Vec<_>>().join("\n");

    let unicode_pattern = r"\p{L}\p{N}\p{P}\p{S}\p{Z}";
    // type only support characters&numbers, others fields support all unicode characters
    let regex_str = format!(
        r"^(?P<type>[\p{{L}}\p{{N}}_-]+)(?:\((?P<scope>[{unicode}]+)\))?!?: (?P<description>[{unicode}]+)$",
        unicode = unicode_pattern
    );

    let re = Regex::new(&regex_str).unwrap();
    const RECOMMENDED_TYPES: [&str; 8] = [
        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor",
    ];

    if let Some(captures) = re.captures(first_line) {
        let commit_type = captures.name("type").map(|m| m.as_str().to_string());
        #[allow(unused_variables)]
        let scope = captures.name("scope").map(|m| m.as_str().to_string());
        let description = captures.name("description").map(|m| m.as_str().to_string());
        if commit_type.is_none() || description.is_none() {
            return false;
        }

        let commit_type = commit_type.unwrap();
        if !RECOMMENDED_TYPES.contains(&commit_type.to_lowercase().as_str()) {
            println!("{} is not a recommended commit type", commit_type);
        }

        // println!("{}({}): {}\n{}", commit_type, scope.unwrap_or("None".to_string()), description.unwrap(), body_footer);

        return true;
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_check_conventional_commits() {
        // successfull cases
        let msg = "feat: add new feature";
        assert!(check_conventional_commits_message(msg));

        let msg = "fix(common crate): bug fix";
        assert!(check_conventional_commits_message(msg));

        let msg = "chore(范围)!: 依存関係を更新する";
        assert!(check_conventional_commits_message(msg));

        let msg = "se_lf-ty9pe(scope)!: Description\n\n여기 시체가 있어요\n\nвот нога";
        assert!(check_conventional_commits_message(msg));

        let msg = "feat(scope)!: Description\n\n\nbody one\n\nbody two\n\nfooter";
        assert!(check_conventional_commits_message(msg));

        // failed casesmsg
        let msg = "feat:add new feature"; // missing ' ' before ':'
        assert!(!check_conventional_commits_message(msg));

        let msg = "fix(common crate)bug fix"; // missing ':'
        assert!(!check_conventional_commits_message(msg));

        let msg = "類@型(common): add new feature"; // unssupported characters in type
        assert!(!check_conventional_commits_message(msg));

        let msg = "()(common): add new feature"; // unssupported characters in type
        assert!(!check_conventional_commits_message(msg));
    }
}
