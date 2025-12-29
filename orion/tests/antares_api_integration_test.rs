use serde_json::{Value, json};
use uuid::Uuid;

const DEFAULT_PATH: &str = "/project";
const DEFAULT_CL: &str = "ILDAJHOI";

#[tokio::test]
async fn test_antares_api_healthcheck() {
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:2725/antares/health")
        .send()
        .await
        .expect("Failed to send health request");

    assert_eq!(response.status(), 200);

    let health: serde_json::Value = response.json().await.unwrap();
    assert_eq!(health["status"], "healthy");
}

#[tokio::test]
async fn test_antares_api_create_mount_success() {
    let client = reqwest::Client::new();

    let request_body = json!({
        "path": DEFAULT_PATH,
        "cl": DEFAULT_CL
    });

    let response = client
        .post("http://localhost:2725/antares/mounts")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send create mount request");

    assert_eq!(response.status(), 200);

    let result: Value = response.json().await.unwrap();
    assert!(!result["mountpoint"].is_null());
    assert!(Uuid::parse_str(result["mount_id"].as_str().unwrap()).is_ok());

    let mount_id = result["mount_id"].as_str().unwrap().to_string();
    let _ = client
        .delete(format!("http://localhost:2725/antares/mounts/{}", mount_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_antares_api_create_mount_with_job_id() {
    let client = reqwest::Client::new();
    let job_id = Uuid::new_v4().to_string();

    let request_body = json!({
        "job_id": job_id,
        "path": DEFAULT_PATH,
        "cl": DEFAULT_CL
    });

    let response = client
        .post("http://localhost:2725/antares/mounts")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send create mount request");

    assert_eq!(response.status(), 200);

    let result: Value = response.json().await.unwrap();
    assert!(!result["mountpoint"].is_null());

    let response = client
        .get(format!(
            "http://localhost:2725/antares/mounts/by-job/{}",
            job_id
        ))
        .send()
        .await
        .expect("Failed to send describe mount by job request");

    assert_eq!(response.status(), 200);
    let status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(status["job_id"], job_id);

    let mount_id = result["mount_id"].as_str().unwrap().to_string();
    let _ = client
        .delete(format!("http://localhost:2725/antares/mounts/{}", mount_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_antares_api_create_mount_validation() {
    let client = reqwest::Client::new();

    let request_body = json!({
        "path": ""
    });

    let response = client
        .post("http://localhost:2725/antares/mounts")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send create mount request");

    assert_eq!(response.status(), 400);

    let error: serde_json::Value = response.json().await.unwrap();
    assert_eq!(error["code"], "INVALID_REQUEST");
}

#[tokio::test]
async fn test_antares_api_duplicate_mount_by_job_id() {
    let client = reqwest::Client::new();
    let job_id = Uuid::new_v4().to_string();

    let request_body = json!({
        "job_id": job_id,
        "path": DEFAULT_PATH,
        "cl": DEFAULT_CL
    });

    let response = client
        .post("http://localhost:2725/antares/mounts")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send create mount request");

    assert_eq!(response.status(), 200);
    let first_result: Value = response.json().await.unwrap();

    let response = client
        .post("http://localhost:2725/antares/mounts")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send duplicate create mount request");

    assert_eq!(response.status(), 200);
    let second_result: Value = response.json().await.unwrap();

    assert_eq!(first_result["mount_id"], second_result["mount_id"]);
    assert_eq!(first_result["mountpoint"], second_result["mountpoint"]);

    let mount_id = first_result["mount_id"].as_str().unwrap().to_string();
    let _ = client
        .delete(format!("http://localhost:2725/antares/mounts/{}", mount_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_antares_api_list_mounts() {
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:2725/antares/mounts")
        .send()
        .await
        .expect("Failed to send list mounts request");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_antares_api_describe_nonexistent_mount() {
    let client = reqwest::Client::new();

    let fake_uuid = Uuid::new_v4();

    let response = client
        .get(format!(
            "http://localhost:2725/antares/mounts/{}",
            fake_uuid
        ))
        .send()
        .await
        .expect("Failed to send describe nonexistent mount request");

    assert_eq!(response.status(), 404);

    let error: serde_json::Value = response.json().await.unwrap();
    assert_eq!(error["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_antares_api_nonexistent_nil_uuid() {
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:2725/antares/mounts/00000000-0000-0000-0000-000000000000")
        .send()
        .await
        .expect("Failed to send invalid UUID request");

    assert_eq!(response.status(), 404);
}
