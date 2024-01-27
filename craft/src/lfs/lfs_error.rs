use std::fmt;

#[derive(Debug)]
pub enum TrackLfsError {
    NotAGitRepository,
    IoError(std::io::Error),
}

impl fmt::Display for TrackLfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TrackLfsError::NotAGitRepository => write!(f, "Not a git repository"),
            TrackLfsError::IoError(ref e) => e.fmt(f),
        }
    }
}

impl std::error::Error for TrackLfsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            TrackLfsError::NotAGitRepository => None,
            TrackLfsError::IoError(ref e) => Some(e),
        }
    }
}

impl From<std::io::Error> for TrackLfsError {
    fn from(err: std::io::Error) -> TrackLfsError {
        TrackLfsError::IoError(err)
    }
}
#[derive(Debug)]
pub enum TestTrackLfsError {
    CurrentDirectoryError(std::io::Error),
    IoError(std::io::Error),
}

impl fmt::Display for TestTrackLfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestTrackLfsError::CurrentDirectoryError(e) => write!(f, "Current directory error: {}", e),
            TestTrackLfsError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for TestTrackLfsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            TestTrackLfsError::CurrentDirectoryError(e) | TestTrackLfsError::IoError(e) => e,
        })
    }
}

impl From<std::io::Error> for TestTrackLfsError {
    fn from(err: std::io::Error) -> TestTrackLfsError {
        TestTrackLfsError::IoError(err)
    }
}