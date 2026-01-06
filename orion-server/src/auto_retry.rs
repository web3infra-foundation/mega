use once_cell::sync::OnceCell;
use std::collections::HashMap;

/// Keywords that indicate transient HTTP or network-related failures in command
/// output. If all of these substrings are observed in the collected output,
/// the failure is treated as retryable even if the exit code alone would not
/// trigger an automatic retry.
const RETRY_KEYWORD: [&str; 3] = ["http", "HTTP", "request"];

pub struct AutoRetryJudger {
    exit_code: i32,
    can_auto_retry_exit_code: OnceCell<bool>,
    retry_keyword_map: HashMap<String, bool>,
}

impl AutoRetryJudger {
    pub fn new() -> Self {
        Self {
            exit_code: 0,
            can_auto_retry_exit_code: OnceCell::new(),
            retry_keyword_map: HashMap::from(
                RETRY_KEYWORD.map(|keyword| (keyword.to_string(), false)),
            ),
        }
    }

    /// it should be true if all output include all RETRY_KEYWORD
    // WARN: optimize this function
    pub fn judge_by_output(&mut self, output: &str) {
        for (keyword, value) in self.retry_keyword_map.iter_mut() {
            if output.contains(keyword) {
                *value = true;
            }
        }
    }

    // should be called one time
    pub fn judge_by_exit_code(&mut self, code: i32) {
        self.exit_code = code;
        if matches!(code, 129..=192) {
            match self.can_auto_retry_exit_code.set(true) {
                Ok(()) => (),
                Err(_) => {
                    tracing::error!(
                        "AutoRetryJudger judge if auto retry by exit code more than once."
                    )
                }
            };
        } else {
            match self.can_auto_retry_exit_code.set(false) {
                Ok(()) => (),
                Err(_) => {
                    tracing::error!(
                        "AutoRetryJudger judge if auto retry by exit code more than once."
                    )
                }
            };
        }
    }

    // can_auto_retry be true, if (can_auto_retry_exit_code) || (all retry_keyword_map.value)
    // if exit code is 0, which means command is succeed, not can auto retry
    pub fn get_can_auto_retry(&self) -> bool {
        if self.exit_code == 0 {
            return false;
        }
        let can_auto_retry_exit_code = match self.can_auto_retry_exit_code.get() {
            Some(o) => *o,
            None => {
                tracing::error!("AutoRetryJudger did not judge exit code.");
                false
            }
        };

        // true: all value is true in map
        let can_auto_retry_keyword = !self.retry_keyword_map.values().any(|value| !*value);

        can_auto_retry_exit_code || can_auto_retry_keyword
    }
}

impl Default for AutoRetryJudger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AutoRetryJudger {
    fn clone(&self) -> Self {
        Self {
            exit_code: self.exit_code,
            can_auto_retry_exit_code: self.can_auto_retry_exit_code.clone(),
            retry_keyword_map: self.retry_keyword_map.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AutoRetryJudger;
    use super::RETRY_KEYWORD;

    macro_rules! not_auto_retry {
        ($exit_code:expr, $output:expr) => {
            !{
                let mut auto_retry_judger = AutoRetryJudger::new();
                auto_retry_judger.judge_by_exit_code($exit_code);
                auto_retry_judger.judge_by_output($output);
                auto_retry_judger.get_can_auto_retry()
            }
        };
    }

    macro_rules! auto_retry {
        ($exit_code:expr, $output:expr) => {{
            let mut auto_retry_judger = AutoRetryJudger::new();
            auto_retry_judger.judge_by_exit_code($exit_code);
            auto_retry_judger.judge_by_output($output);
            auto_retry_judger.get_can_auto_retry()
        }};
    }

    macro_rules! get_auto_retry_with_array_output {
        ($exit_code:expr, $output_array:ident) => {{
            let mut auto_retry_judger = AutoRetryJudger::new();
            auto_retry_judger.judge_by_exit_code($exit_code);
            for output in $output_array {
                auto_retry_judger.judge_by_output(output);
            }
            auto_retry_judger.get_can_auto_retry()
        }};
    }

    #[test]
    fn test_auto_retry_judger() {
        // correct exit code
        //   with all output keys
        // it should be "not can auto retry"
        let exit_code = 0;
        let output = RETRY_KEYWORD.join(" ");
        assert!(not_auto_retry!(exit_code, &output));
        //  with a part of retry keys
        // it should be "not can auto retry"
        let output = RETRY_KEYWORD[0];
        assert!(not_auto_retry!(exit_code, &output));
        // with none keys
        // it should be "not can auto retry"
        let output = "do not include any keywords";
        assert!(not_auto_retry!(exit_code, &output));

        // not correct and not signal exit code
        //   with all output keys
        // it should be "can auto retry"
        let exit_code = 1;
        let output = RETRY_KEYWORD.join(" ");
        assert!(auto_retry!(exit_code, &output));
        //   with a part of retry keys
        // it should be "not can auto retry"
        let output = RETRY_KEYWORD[0];
        assert!(not_auto_retry!(exit_code, &output));
        //   with none retry keys
        // it should be "not can auto retry"
        let output = "";
        assert!(not_auto_retry!(exit_code, &output));

        // signal interruption exit code
        //   with all output keys
        // it should be "can auto retry"
        let exit_code = 130;
        let output = RETRY_KEYWORD.join(" ");
        assert!(auto_retry!(exit_code, &output));
        //   with a part of retry keys
        // it should be "can auto retry"
        let output = RETRY_KEYWORD[0];
        assert!(auto_retry!(exit_code, &output));
        //   with none retry keys
        // it should be "can auto retry"
        let output = "";
        assert!(auto_retry!(exit_code, &output));
    }

    #[test]
    fn test_auto_retry_judger_with_multioutput() {
        // correct exit code
        //   with all output keys
        // it should be "not can auto retry"
        let exit_code = 0;
        let output = RETRY_KEYWORD.join(" ");
        assert_eq!(
            auto_retry!(exit_code, &output),
            get_auto_retry_with_array_output!(exit_code, RETRY_KEYWORD)
        );

        // not correct and not signal exit code
        //   with all output keys
        // it should be "can auto retry"
        let exit_code = 1;
        let output = RETRY_KEYWORD.join(" ");
        assert_eq!(
            auto_retry!(exit_code, &output),
            get_auto_retry_with_array_output!(exit_code, RETRY_KEYWORD)
        );

        // signal interruption exit code
        //   with all output keys
        // it should be "can auto retry"
        let exit_code = 130;
        let output = RETRY_KEYWORD.join(" ");
        assert_eq!(
            auto_retry!(exit_code, &output),
            get_auto_retry_with_array_output!(exit_code, RETRY_KEYWORD)
        );
    }
}
