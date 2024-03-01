#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn remove_trailing_newlines(input: String) -> String {
    input.trim_end_matches('\n').to_string()
}
#[cfg(target_os = "windows")]
pub fn remove_trailing_newlines(input: String) -> String {
    if input.ends_with("\r\n") {
        input.trim_end_matches("\r\n").to_string()
    } else {
        input.trim_end_matches('\n').to_string()
    }
}