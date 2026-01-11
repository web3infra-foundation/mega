pub fn base_url() -> String {
    let mut base = std::env::var("SCORPIO_API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:2725".to_string());

    if !base.contains("://") {
        base = format!("http://{base}");
    }
    while base.ends_with('/') {
        base.pop();
    }
    base
}
