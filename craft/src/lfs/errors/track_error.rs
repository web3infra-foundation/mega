use std::error::Error;
use std::fmt;
use std::io;
define_error!(GitAttributesError);
impl From<GitAttributesError> for DefaultGitAttributesError {
    fn from(err: GitAttributesError) -> Self {
        let error_message = format!("GitAttributes occurred: {}", err);
        DefaultGitAttributesError::with_source(error_message,err)
    }
}
define_error!(GitRepositoryCheckerError);
define_error!(DefaultGitAttributesError);
