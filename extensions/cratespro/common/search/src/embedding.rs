use pgvector::Vector;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use tokio_postgres::Client as PgClient;

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

pub async fn get_one_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    get_texts_embedding(&[text])
        .await
        .map(|embeddings| embeddings[0].clone())
}

//TODO 1: 优化get_texts_embedding函数，使其使用batch API
async fn get_texts_embedding(texts: &[&str]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = Client::new();
    let open_ai_embedding_url =
        env::var("OPEN_AI_EMBEDDING_URL").expect("OPEN_AI_EMBEDDING_URL not set");

    // let url = "https://api.xty.app/v1/embeddings";
    let request_body = json!({
        "input": texts,
        "model": "text-embedding-3-small"
    });
    let response = client
        .post(open_ai_embedding_url)
        .header("Content-Type", "application/json")
        .header("AUTHORIZATION", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?
        .json::<EmbeddingResponse>()
        .await?;

    let embeddings: Vec<Vec<f32>> = response.data.into_iter().map(|d| d.embedding).collect();

    Ok(embeddings)
}

pub async fn update_crate_embeddings(
    client: &PgClient,
    crate_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME not set");
    let query = format!("SELECT name, description FROM {table_name} WHERE id = $1");
    let row = client
        .query_one(&query, &[&crate_id])
        .await
        .expect("Failed to get crate");
    let name: String = row.get("name");
    let description: String = row.get("description");
    let text = format!("crate name:{name}, crate description:{description}");
    let embedding = get_one_text_embedding(&text).await.unwrap();
    let embedding = Vector::from(embedding);
    let update_query = format!("UPDATE {table_name} SET embedding = $1 WHERE id = $2");
    client
        .execute(&update_query, &[&embedding, &crate_id])
        .await
        .expect("Failed to update crate embedding");
    Ok(())
}

pub async fn update_all_crate_embeddings(
    client: &PgClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME not set");
    let query = format!("SELECT id, name, description FROM {table_name}");
    let rows = client.query(&query, &[]).await?;
    let texts: Vec<String> = rows
        .iter()
        .map(|row| {
            let name: Option<&str> = row.get("name");
            let description: Option<&str> = row.get("description");
            format!(
                "crate name:{}, crate description:{}",
                name.unwrap_or(""),
                description.unwrap_or("")
            )
        })
        .collect();
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let mut all_embeddings = Vec::new();
    for chunk in text_refs.chunks(32) {
        let embeddings = get_texts_embedding(chunk).await?;
        println!("Got embeddings for {} texts", embeddings.len());
        all_embeddings.extend(embeddings);
    }

    // println!("{:?}", all_embeddings);
    for (i, row) in rows.iter().enumerate() {
        let id: &str = row.get("id");
        let embedding = Vector::from(all_embeddings[i].clone());
        let update_query = format!("UPDATE {table_name} SET embedding = $1 WHERE id = $2");
        client.execute(&update_query, &[&embedding, &id]).await?;
        println!("Updated embedding for crate {id}");
    }
    Ok(())
}

// 新增函数
pub(crate) async fn search_crates_by_embedding(
    client: &PgClient,
    embedding: &[f32],
    n: i64,
) -> Result<Vec<(i32, String, String)>, Box<dyn std::error::Error>> {
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME not set");
    let query = format!(
        "SELECT id, name, description FROM {table_name} ORDER BY embedding <=> $1 LIMIT $2"
    );
    let rows = client.query(&query, &[&embedding, &n]).await?;

    let results = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get("id");
            let name: String = row.get("name");
            let description: String = row.get("description");
            (id, name, description)
        })
        .collect();

    Ok(results)
}
