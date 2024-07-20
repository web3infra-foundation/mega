pub mod cli;
mod commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli() {
        let config_path = "config.toml";
        let args = vec!["-c", config_path, "service", "multi", "http"];
        cli::parse(Some(args)).expect("Failed to start http service");
    }
}