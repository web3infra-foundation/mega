use crate::task::Content;

use std::collections::HashMap;

pub type Variable = Content;

/// # Environment variable.
///
/// When multiple tasks are running, they may need to share the same data or read
/// the same configuration information. Environment variables can meet this requirement.
/// Before all tasks run, the user builds a [`EnvVar`] and sets all the environment
/// variables. One [`EnvVar`] corresponds to one dag. All tasks in a job can
/// be shared and immutable at runtime. environment variables.
#[derive(Debug, Default)]
pub struct EnvVar {
    variables: HashMap<String, Variable>,
}

impl EnvVar {
    /// Allocate a new [`EnvVar`].
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    #[allow(unused)]
    /// Set a global variables.
    ///
    /// # Example
    /// ```rust
    /// # let mut env = dagrs::EnvVar::new();
    /// env.set("Hello", "World".to_string());
    /// ```
    pub fn set<H: Send + Sync + 'static>(&mut self, name: &str, var: H) {
        let mut v = Variable::new(var);
        self.variables.insert(name.to_owned(), v);
    }

    /// Get environment variables through keys of type &str.
    ///
    /// Note: This method will clone the value. To avoid cloning, use [`get_ref`].
    pub fn get<H: Send + Sync + Clone + 'static>(&self, name: &str) -> Option<H> {
        self.get_ref(name).cloned()
    }

    /// Get environment variables through keys of type &str.
    pub fn get_ref<H: Send + Sync + 'static>(&self, name: &str) -> Option<&H> {
        if let Some(content) = self.variables.get(name) {
            content.get()
        } else {
            None
        }
    }
}
