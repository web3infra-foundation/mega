use serial_test::serial;

#[tokio::test]
#[serial]
async fn check_mono_service_status() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8000/api/v1/status")
        .send()
        .await?;

    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Service status API did not return 200 OK"
    );
    Ok(())
}
