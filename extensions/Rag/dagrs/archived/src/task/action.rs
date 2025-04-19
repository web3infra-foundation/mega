use crate::{EnvVar, Input, Output};
use std::sync::Arc;

/// The type of closure that performs logic.
/// # [`Simple`]
///
/// The specific type of [`Simple`] is `dyn Fn(Input, Arc<EnvVar>) -> Output + Send + Sync`,
/// which represents a closure.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use dagrs::{Action,Input,EnvVar,Output};
///
/// let closure=|_input,_env|Output::new(10);
/// let action=Action::Closure(Arc::new(closure));
/// ```
pub type Simple = dyn Fn(Input, Arc<EnvVar>) -> Output + Send + Sync;

/// More complex types of execution logic.
/// # [`Complex`]
///
/// The usage of closures is suitable for simple cases. If the user wants to store some private
/// properties when defining execution logic, the [`Complex`] trait can meet the needs.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use dagrs::{Action,Input,EnvVar,Output,Complex};
///
/// struct HelloAction{
///    statement: String,
///    repeat: usize,
/// }
///
/// impl Complex for HelloAction{
///     fn run(&self, input: Input, env: Arc<EnvVar>) -> Output{
///         for i in 0..self.repeat {
///             println!("{}",self.statement);
///         }
///         Output::empty()
///     }
/// }
///
/// let hello=HelloAction {
///     statement: "hello world!".to_string(),
///     repeat: 10
/// };
/// let action = Action::Structure(Arc::new(hello));
/// ```
pub trait Complex {
    fn run(&self, input: Input, env: Arc<EnvVar>) -> Output;
}

/// Task specific behavior
///
/// [`Action`] stores the specific execution logic of a task. Action::Closure(Arc<[`Simple`]>) represents a
/// closure, and Action::Structure(Arc<dyn Complex + Send + Sync>) represents a specific type that
/// implements the [`Complex`] trait.
/// Attributes that must exist in each task are used to store specific execution logic. Specific
/// execution logic can be given in two forms: given a closure or a specific type that implements
/// a Complex trait.
#[derive(Clone)]
pub enum Action {
    Closure(Arc<Simple>),
    Structure(Arc<dyn Complex + Send + Sync>),
}

impl Action {
    pub fn run(&self, input: Input, env: Arc<EnvVar>) -> Output {
        match self {
            Self::Closure(closure) => closure(input, env),
            Self::Structure(structure) => structure.run(input, env),
        }
    }
}
