use idgenerator::IdInstance;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
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
