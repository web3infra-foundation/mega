use once_cell::sync::OnceCell;

/// Mono base URL for file/blob API, set at startup from config (or default).
static MONO_BASE_URL: OnceCell<String> = OnceCell::new();

/// Set the mono base URL from config. Call once at server startup.
pub fn set_mono_base_url(url: String) {
    let _ = MONO_BASE_URL.set(url);
}
