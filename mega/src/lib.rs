pub mod cli;
mod commands;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli() {
        let args = "service http".split(' ').collect();
        cli::parse(Some(args)).expect("Failed to start http service");
    }
}