use std::error::Error;
use std::fmt;
use std::io;
#[macro_export]
macro_rules! define_error {
    ($name:ident) => {
        #[derive(Debug)]
        pub struct $name {
            message: String,
            source: Option<Box<dyn Error + Send + Sync>>,
        }

        impl $name {
            pub fn new<M: Into<String>>(message: M) -> Self {
                Self {
                    message: message.into(),
                    source: None,
                }
            }

            pub fn with_source<M: Into<String>, E: Error + Send + Sync + 'static>(
                message: M,
                source: E,
            ) -> Self {
                Self {
                    message: message.into(),
                    source: Some(Box::new(source)),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.message)
            }
        }

        impl Error for $name {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                self.source.as_ref().map(|b| &**b as &(dyn Error + 'static))
            }
        }
        impl From<io::Error> for $name {
            fn from(err: io::Error) -> Self {
                $name::new(err.to_string())
            }
        }
    };
}