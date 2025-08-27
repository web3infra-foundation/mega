use crate::model::{ReviewRequest, ReviewSuggestRes};
use axum::{extract::Json, response::Json as RespJson, routing::post, Router};
use chat::{self, generate_suggestion, search_context};

/// 提取带上下文的代码变更（包含修改前和修改后）
/// 返回格式: Vec<(文件路径, 变更类型, 旧代码, 新代码)>
fn extract_code_chunks_with_context(
    diff_output: &str,
) -> Vec<(String, &'static str, String, String)> {
    let mut result = Vec::new();
    let chunks: Vec<&str> = diff_output.split("diff --git ").collect();

    for chunk in chunks.iter().skip(1) {
        // 解析文件路径
        let file_path = chunk
            .lines()
            .next()
            .and_then(|s| s.split(' ').nth(1))
            .map(|s| s.trim_start_matches("a/").to_string())
            .unwrap_or_default();

        // 跳过删除文件和二进制文件
        if chunk.contains("deleted file mode") || chunk.contains("Binary files") {
            continue;
        }

        let is_new_file = chunk.contains("new file mode");
        let change_type = if is_new_file { "NEW_FILE" } else { "MODIFIED" };

        // 提取带上下文的变更
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();
        let mut in_hunk = false;
        let mut hunk_header = String::new();

        for line in chunk.lines() {
            let line = line.trim_start();
            let trimmed_line = line.trim_end();
            match trimmed_line {
                l if l.starts_with("@@") => {
                    in_hunk = true;
                    hunk_header = l.to_string();
                }
                l if in_hunk => {
                    match l.chars().next() {
                        Some('-') => old_lines.push(&l[1..]), // 删除行（保留原始缩进）
                        Some('+') => new_lines.push(&l[1..]), // 新增行（保留原始缩进）
                        _ => {
                            old_lines.push(&l[0..]);
                            new_lines.push(&l[0..]);
                        }
                    }
                }
                _ => {}
            }
        }

        if !new_lines.is_empty() {
            result.push((
                file_path,
                change_type,
                format!("{}\n{}", hunk_header, old_lines.join("\n")),
                format!("{}\n{}", hunk_header, new_lines.join("\n")),
            ));
        }
    }

    result
}

async fn review_suggest(Json(payload): Json<ReviewRequest>) -> RespJson<ReviewSuggestRes> {
    let changes = extract_code_chunks_with_context(&payload.diff);
    let mut suggestions = Vec::new();
    for (path, change_type, old_code, new_code) in changes {
        let query = match change_type {
            "NEW_FILE" => format!("{}", new_code),
            "MODIFIED" => format!("Modified code:\n{}", new_code),
            _ => continue,
        };
        let context = match search_context(&query).await {
            Ok(content) => content,
            Err(_) => String::new(),
        };
        let prompt = format!(
            "You are a code reviewer.\nThe code content to be reviewed is as follows:\n{}\n\nThe following is the context retrieved by RAG (may or may not be helpful):\n{}",
            query, context
        );
        let mut suggestion = String::new();
        match generate_suggestion(&prompt).await {
            Ok(s) => suggestion.push_str(&format!("{}", s)),
            Err(e) => (),
        }
        suggestions.push(suggestion);
    }
    RespJson(ReviewSuggestRes { suggestions })
}

pub fn router() -> Router {
    Router::new().route("/review/suggest", post(review_suggest))
}

#[cfg(test)]
mod test {
    use crate::api::extract_code_chunks_with_context;

    #[test]
    fn test_parse_diff_result_to_filelist() {
        let diff_output = r#"
        diff --git a/src/main.rs b/src/main.rs
        index abc123..def456 100644
        --- a/src/main.rs
        +++ b/src/main.rs
        @@ -5,7 +5,8 @@
        fn main() {
            let x = 10;
        -    println!("Old");  // 删除行
        +    println!("New");  // 新增行
        +    println!("Added"); // 新增行
        }

        diff --git a/src/new.rs b/src/new.rs
        new file mode 100644
        index 0000000..d1a2b3c
        --- /dev/null
        +++ b/src/new.rs
        @@ -0,0 +1,3 @@
        +// 新文件
        +fn hello() {
        +    println!("New file");
        +}

        diff --git a/src/old.rs b/src/old.rs
        deleted file mode 100644
        index e1f2a3b..0000000
        --- a/src/old.rs
        +++ /dev/null
        @@ -1,3 +0,0 @@
        -// 被删除的文件
        -fn old() {
        -    println!("Deleted");
        -}
        "#;
        let changes = extract_code_chunks_with_context(diff_output);
        for (path, change_type, code, new_code) in changes {
            println!("File: {} ({})", path, change_type);
            println!("{}", code);
            println!("{}", new_code);
            println!("-----");
        }
    }
}
