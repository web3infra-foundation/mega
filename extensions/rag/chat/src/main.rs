use axum::{routing::post, Json, Router};
use chat::generation::GenerationNode;
use chat::search::SearchNode;
use chat::{broker, consumer_group, llm_url, qdrant_url, topic, vect_url};
use futures::StreamExt;
use log::{error, info};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::CommitMode;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::error::Error;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
pub struct CveId {
    pub id: String,
    pub url: String,
}

#[derive(Deserialize)]
struct ChatRequest {
    prompt: String,
}

#[derive(Deserialize)]
struct CodeRequest {
    message: String,
    code: String,
}

#[allow(dead_code)]
#[derive(Deserialize, sqlx::FromRow)]
struct RustsecInfo {
    id: String,
    subtitle: String,
    reported: String,
    issued: String,
    package: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    r#type: String,
    keywords: String,
    aliases: String,
    reference: String,
    patched: String,
    unaffected: String,
    description: String,
    affected: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct CveAnalyzeRes {
    crate_version: String,
    dept_crate_version: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepTriggerResult {
    crate_version: String,
    dep: String,
    trigger: bool,
    reason: String,
}

// #[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
// struct CveFullAnalysis {
//     id: String,
//     vuln_func: String,
//     dep_results: serde_json::Value,
// }

#[allow(dead_code)]
#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    // 创建数据库连接池
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://mega:mega@10.42.0.1:31432/cratespro".to_string());

    // 将数据库URL解析为postgreSQL连接配置对象
    let connect_options = sqlx::postgres::PgConnectOptions::from_str(&database_url)
        .map_err(|e| format!("Failed to parse DATABASE_URL: {}", e))?;

    // 创建连接池，目前设置最大连接数为10
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await
        .map_err(|e| format!("DB connect error: {}", e))?;

    info!("Database connection pool established.");

    init_db(&pool).await?;

    //把数据库连接池打包成应用状态，方便传给所有 HTTP handler
    let app_state = AppState { pool: pool.clone() };

    let app = Router::new()
        .route("/chat", post(chat_handler))
        .route("/code", post(code_handler))
        .route("/debug", post(debug_handler))
        .with_state(app_state);

    info!("Server running on http://0.0.0.0:30088");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:30088").await?;

    // spawn consumer
    let consumer_handle = tokio::spawn({
        let pool = pool.clone();
        async move {
            if let Err(e) = start_cve_consumer(pool).await {
                error!("CVE consumer exited with error: {}", e);
                return Err(e);
            }
            Ok(())
        }
    });

    // axum server future
    let server = axum::serve(listener, app);

    tokio::select! {
        res = server => {
            if let Err(e) = res {
                error!("Axum server error: {}", e);
                return Err(e.into());
            }
        }
        res = consumer_handle => {
            match res {
                Ok(Ok(())) => {
                    info!("CVE consumer exited normally");
                }
                Ok(Err(e)) => {
                    error!("CVE consumer failed: {}", e);
                    return Err(e);
                }
                Err(join_err) => {
                    error!("CVE consumer panic: {}", join_err);
                    return Err(join_err.into());
                }
            }
        }
    }

    Ok(())
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cve_full_analysis (
            id TEXT PRIMARY KEY,
            vuln_func TEXT,
            dep_results JSONB,
            created_at TIMESTAMP DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn start_cve_consumer(pool: PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let group_id = consumer_group();
    let kafka_broker = broker();
    let topic = topic();

    let max_poll_interval_ms = "18000000"; // 5小时
    let session_timeout_ms = "60000";

    info!("Initializing CVE Kafka consumer (serial mode)...");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &group_id)
        .set("bootstrap.servers", &kafka_broker)
        .set("max.poll.interval.ms", max_poll_interval_ms)
        .set("session.timeout.ms", session_timeout_ms)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&[&topic])?;
    info!("CVE Kafka consumer started (serial mode)");

    let mut message_stream = consumer.stream();

    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(borrowed_message) => {
                let payload = match borrowed_message.payload_view::<str>() {
                    Some(Ok(s)) => s.to_string(),
                    Some(Err(e)) => {
                        error!("Invalid UTF-8 payload: {}. Skipping.", e);
                        continue;
                    }
                    None => {
                        info!("Empty payload, skipping.");
                        continue;
                    }
                };
                info!("📨 Received CVE message: {}", payload);
                // JSON 解析
                let cveid = match serde_json::from_str::<CveId>(&payload) {
                    Ok(cveid) => cveid,
                    Err(e) => {
                        error!("Failed to parse JSON: {}", e);
                        continue;
                    }
                };

                let id_clone = cveid.id.clone();
                info!("🚀 [START] Processing {}", id_clone);

                // 串行处理消息
                match process_cveid_and_analyze(cveid, pool.clone()).await {
                    Ok(msg) => info!("✅ [SUCCESS] {} processed: {}", id_clone, msg),
                    Err(e) => error!("❌ [FAIL] {} failed: {}", id_clone, e),
                }

                // ✅ 处理完成后再提交 offset
                if let Err(e) = consumer.commit_message(&borrowed_message, CommitMode::Async) {
                    error!("Commit offset failed for {}: {}", id_clone, e);
                }
            }
            Err(e) => error!("Kafka stream error: {}", e),
        }
    }

    Ok(())
}

async fn process_cveid_and_analyze(cveid: CveId, pool: PgPool) -> Result<String, String> {
    info!("🔎 [CVEId] Received id: {}, url: {}", cveid.id, cveid.url);

    let row = sqlx::query_as::<_, RustsecInfo>("SELECT id, subtitle, reported, issued, package, type, keywords, aliases, reference, patched, unaffected, description, affected FROM rustsec_info WHERE id = $1")
        .bind(&cveid.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("DB query error: {}", e))?;

    match row {
        Some(rustsec_info) => {
            info!(
                "✅ [CVEId] Found RustsecInfo for id: {}. Starting full analysis...",
                cveid.id
            );
            perform_cve_analysis(rustsec_info, &pool).await
        }
        None => {
            error!("❌ [CVEId] No RustsecInfo found for id: {}", cveid.id);
            Err(format!("No RustsecInfo found for id: {}", cveid.id))
        }
    }
}

pub fn extract_rust_code_block(s: &str) -> Option<String> {
    // 优先匹配 ```rust ... ```
    if let Some(start) = s.find("```rust") {
        if let Some(rest) = s[start + "```rust".len()..].find("```") {
            let code = &s[start + "```rust".len()..start + "```rust".len() + rest];
            return Some(code.trim().to_string());
        }
    }
    // 次选匹配第一个 ``` ... ```
    if let Some(start) = s.find("```") {
        let tail = &s[start + 3..];
        if let Some(end_rel) = tail.find("```") {
            let code = &tail[..end_rel];
            return Some(code.trim().to_string());
        }
    }
    None
}

async fn perform_cve_analysis(payload: RustsecInfo, pool: &PgPool) -> Result<String, String> {
    info!("🟢 [START] Received CVE request: {}", payload.id);

    // === 1️⃣ 生成漏洞函数上下文 ===
    // let cve_context = format!(
    //     r#"You are an automated Rust code extractor.
    //     Your single and only task is to extract *only* the specific vulnerable function code based on the CVE data.

    //     **CRITICAL INSTRUCTIONS:**
    //     1.  DO NOT provide any explanation, analysis, or introductory text (like "Here is the code:").
    //     2.  DO NOT provide the fixed or corrected version of the code.
    //     3.  Your response MUST contain *only* the raw, **VULNERABLE** Rust function code.
    //     4.  Do not wrap the code in Markdown (like ```rust).
    //     5.  Start your response directly with `fn` or `pub fn`.

    //     **CVE Data:**
    //     ID: {}
    //     Description: {}
    //     Affected versions: {}

    //     **Vulnerable Function Code Only:**
    //     "#,
    //     payload.id, payload.description, payload.affected
    // );
    let cve_context = format!(
        "You are a Rust security expert.
        CVE ID: {id}
        Description: {desc}
        Affected versions: {affected}

        Task:
        - Provide the exact vulnerable function that causes this CVE.

        Output rules (must follow strictly):
        - Return ONLY ONE fenced code block in Rust:
        ```rust
        fn ... {{ ... }}
        ```
        - No extra text, no markdown outside the single fenced code block.
        - Do not include any explanations, comments, or thoughts.",
        id = payload.id,
        desc = payload.description,
        affected = payload.affected
    );
    info!("📄 [LLM Prompt] CVE Context built for ID: {}", payload.id);
    let generation_node = GenerationNode::new(&llm_url(), None);
    let raw_vuln_func = match generation_node.generate(&cve_context).await {
        Ok(msg) => {
            info!(
                "✅ [LLM] Vulnerable function analysis generated successfully for {}",
                payload.id
            );
            msg
        }
        Err(e) => {
            error!("❌ [LLM ERROR] Generation failed for {}: {}", payload.id, e);
            return Err(format!("Function analysis failed: {}", e));
        }
    };
    let vuln_func =
        extract_rust_code_block(&raw_vuln_func).unwrap_or_else(|| raw_vuln_func.trim().to_string());

    // === 3️⃣ 查询分析结果 ===
    let row: Option<(String,)> =
        sqlx::query_as::<_, (String,)>("SELECT res FROM cve_analysis_res WHERE id = $1")
            .bind(&payload.id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("DB query error: {}", e))?;

    let analysis_res: Vec<CveAnalyzeRes> = if let Some((json_str,)) = row {
        serde_json::from_str(&json_str).map_err(|e| format!("Deserialize error: {}", e))?
    } else {
        error!(
            "No analysis result found for {}. Task will be retried.",
            payload.id
        );
        return Err(format!("No analysis result found for {}", payload.id));
    };

    // === 4️⃣ 遍历依赖进行触发检测 ===
    let mut dep_results = Vec::<DepTriggerResult>::new();
    for (i, res) in analysis_res.iter().enumerate() {
        info!("🧩 [ITEM {}] Checking crate: {}", i + 1, res.crate_version);
        for dep in &res.dept_crate_version {
            let check_prompt = format!(
                "You are a Rust dependency analyzer.
                 The following vulnerable function has been identified:
                 {func}

                 Dependency chain: {root} depends on {dep}

                 Question: Based on this vulnerable function, is it possible that dependency {dep}
                 would trigger the vulnerability?

                 Output rules (strict):
                 - Return only ONE minified JSON object in ONE line.
                 - No markdown, no backticks, no explanations.
                 - Fields:
                   {{\"dep\":\"{dep}\",\"trigger\":true|false,\"reason\":\"short explanation\"}}",
                func = vuln_func,
                root = res.crate_version,
                dep = dep
            );
            let check_node = GenerationNode::new(&llm_url(), None);
            match check_node.generate(&check_prompt).await {
                Ok(resp) => {
                    let parsed_json = serde_json::from_str::<serde_json::Value>(&resp)
                        .ok()
                        .or_else(|| {
                            extract_first_json_object(&resp)
                                .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
                        });

                    if let Some(value) = parsed_json {
                        let dep_name = value
                            .get("dep")
                            .and_then(|v| v.as_str())
                            .unwrap_or(dep)
                            .to_string();
                        let trigger_bool = match value.get("trigger") {
                            Some(t) if t.is_boolean() => t.as_bool().unwrap(),
                            Some(t) if t.is_string() => {
                                matches!(
                                    t.as_str().unwrap_or_default().to_lowercase().as_str(),
                                    "true" | "yes" | "y"
                                )
                            }
                            _ => false,
                        };
                        let reason_text = value
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        dep_results.push(DepTriggerResult {
                            crate_version: res.crate_version.clone(),
                            dep: dep_name,
                            trigger: trigger_bool,
                            reason: reason_text,
                        });
                    } else {
                        dep_results.push(DepTriggerResult {
                            crate_version: res.crate_version.clone(),
                            dep: dep.clone(),
                            trigger: false,
                            reason: resp.replace('\n', " ").chars().take(300).collect(),
                        });
                    }
                }
                Err(e) => {
                    dep_results.push(DepTriggerResult {
                        crate_version: res.crate_version.clone(),
                        dep: dep.clone(),
                        trigger: false,
                        reason: e.to_string(),
                    });
                }
            }
        }
    }

    // === 5️⃣ 插入结构化结果 ===
    let dep_json = serde_json::to_value(&dep_results)
        .map_err(|e| format!("Serialize dep_results error: {}", e))?;
    let insert_query = r#"
        INSERT INTO cve_full_analysis (id, vuln_func, dep_results)
        VALUES ($1, $2, $3)
        ON CONFLICT (id) DO UPDATE
        SET vuln_func = EXCLUDED.vuln_func,
            dep_results = EXCLUDED.dep_results,
            created_at = NOW();
    "#;
    sqlx::query(insert_query)
        .bind(&payload.id)
        .bind(&vuln_func)
        .bind(&dep_json)
        .execute(pool)
        .await
        .map_err(|e| format!("Insert error: {}", e))?;
    info!("✅ [DB INSERT] Structured CVE analysis result saved.");

    // === 6️⃣ 返回结果给调用者 ===
    Ok(format!(
        "✅ CVE {} full structured analysis completed and stored.",
        payload.id
    ))
}

fn extract_first_json_object(s: &str) -> Option<String> {
    // 提取第一个JSON对象
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'{' {
            if start.is_none() {
                start = Some(i);
            }
            depth += 1;
        } else if b == b'}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                if let Some(st) = start {
                    return Some(String::from_utf8_lossy(&bytes[st..=i]).to_string());
                }
            }
        }
    }
    None
}

// POST /chat
async fn chat_handler(Json(payload): Json<ChatRequest>) -> Result<Json<String>, String> {
    info!("Received chat request: {}", payload.prompt);

    // Create SearchNode with request prompt
    let search_node = SearchNode::new(
        &vect_url(),
        &qdrant_url(),
        "test_test_code_items",
        &payload.prompt,
    )
    .expect("Failed to create SearchNode");

    // Execute search directly
    let search_result = match search_node.search(&payload.prompt).await {
        Ok(Some((content, item_type))) => {
            info!(
                "Search result found: type={}, content length={}",
                item_type,
                content.len()
            );
            info!("Search content: {}", content);
            format!(
                "{}\nThe enhanced information after local RAG may be helpful, but it is not necessarily accurate:\n Related information type: {}\nRelated information Content: {}",
                payload.prompt,
                item_type,
                content
            )
        }
        Ok(None) => {
            info!("No search results found");
            payload.prompt
        }
        Err(e) => {
            error!("Search error: {}", e);
            return Err(format!("Search failed: {}", e));
        }
    };

    info!("Search result for generation: {}", search_result);

    // Create GenerationNode and execute generation
    let generation_node = GenerationNode::new(&llm_url(), None); // No oneshot needed for direct execution
    let generated_message = match generation_node.generate(&search_result).await {
        Ok(msg) => {
            info!("Generation completed successfully");
            msg
        }
        Err(e) => {
            error!("Generation error: {}", e);
            return Err(format!("Generation failed: {}", e));
        }
    };

    info!("Final response: {}", generated_message);
    Ok(Json(generated_message))
}

async fn code_handler(Json(payload): Json<CodeRequest>) -> Result<Json<String>, String> {
    info!("Received chat code: {}", payload.code);
    info!("Received chat request: {}", payload.message);

    // Create SearchNode with request prompt
    let search_node = SearchNode::new(
        &vect_url(),
        &qdrant_url(),
        "test_test_code_items",
        &payload.code,
    )
    .expect("Failed to create SearchNode");

    // Execute search directly
    let search_result = match search_node.search(&payload.code).await {
        Ok(Some((content, item_type))) => {
            info!(
                "Search result found: type={}, content length={}",
                item_type,
                content.len()
            );
            format!(
                "RAG-enhanced related information (may be helpful but not necessarily accurate):\n\
                 Related information type: {}\n\
                 Content: {}",
                item_type, content
            )
        }
        Ok(None) => {
            info!("No search results found");
            String::from("No RAG-enhanced information found.")
        }
        Err(e) => {
            error!("Search error: {}", e);
            return Err(format!("Search failed: {}", e));
        }
    };

    info!("Search result for generation: {}", search_result);

    // Combine message + code + search result into a dedicated prompt
    let dedicated_prompt = format!(
        "You are an expert Rust developer and code reviewer.\n\nUser Message:\n{}\n\nInput Code:\n{}\n\nRelated Information:\n{}\n\nInstructions:\n- Analyze the code above in the context of the user's message.\n- Identify any issues, bugs, or potential improvements.\n- Provide clear explanations or suggestions.\n- Optionally, give a corrected or optimized version if needed.\n\nYour response should be concise, informative, and actionable.",
        payload.message,  // 用户的说明文字
        payload.code,     // 代码内容
        search_result     // 本地搜索增强结果
    );

    // Pass the dedicated prompt to GenerationNode
    let generation_node = GenerationNode::new(&llm_url(), None); // No oneshot needed for direct execution
    let generated_message = match generation_node.generate(&dedicated_prompt).await {
        Ok(msg) => msg,
        Err(e) => {
            error!("Generation error: {}", e);
            return Err(format!("Generation failed: {}", e));
        }
    };
    info!("Final response: {}", generated_message);
    Ok(Json(generated_message))
}

async fn debug_handler(Json(payload): Json<serde_json::Value>) -> Result<Json<String>, String> {
    info!("Debug JSON received: {:?}", payload);

    let id = payload["id"].as_str().unwrap_or("N/A");
    let summary = payload["summary"].as_str().unwrap_or("N/A");
    let details = payload["details"].as_str().unwrap_or("N/A");

    let dedicated_prompt = format!(
        "You are a Rust security expert.\n\n\
         Advisory ID: {}\n\n\
         Summary:\n{}\n\n\
         Details:\n{}\n\n\
         Instructions:\n\
         - Explain the vulnerability in simple terms.\n\
         - Identify the root cause.\n\
         - Discuss possible impact.\n\
         - Suggest mitigation and best practices.",
        id, summary, details
    );

    let generation_node = GenerationNode::new(&llm_url(), None);
    let generated_message = match generation_node.generate(&dedicated_prompt).await {
        Ok(msg) => {
            info!("Generation completed successfully");
            msg
        }
        Err(e) => {
            error!("Generation error: {}", e);
            return Err(format!("Generation failed: {}", e));
        }
    };

    Ok(Json(generated_message))
}
