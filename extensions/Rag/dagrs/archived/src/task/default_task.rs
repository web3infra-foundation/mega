use super::{Action, Complex, Task, ID_ALLOCATOR};
use crate::{EnvVar, Input, Output};
use std::sync::Arc;

/// Common task types
///
/// [`DefaultTask`] is a default implementation of the [`Task`] trait. Users can use this task
/// type to build tasks to meet most needs.
///
/// There are four ways to create a DefaultTask:
///
/// #Example
///
/// Using the `default` function to create a task is the simplest way. The name of the task defaults
/// to "Task $id", and a closure without output is created by default as the execution logic of
/// the task. Subsequently, users can specify the task name through the `set_name` function, and
/// use the `set_action` or `set_closure` function to specify execution logic for the task.
///
/// ```rust
/// use dagrs::DefaultTask;
/// let mut task=DefaultTask::default();
/// ```
/// Use the `new` function to create a task. The only difference between this function and the `default`
/// function is that the task name is also given when creating the task. The subsequent work is the
/// same as mentioned in the `default` function.
///
/// ```rust
/// use dagrs::DefaultTask;
/// let mut task=DefaultTask::new("task");
/// ```
/// To build task execution logic, please see `action` module.
///
/// Use the `with_closure` function to create a task and give it a task name and execution logic.
/// The execution logic is given in the form of a closure.
///
/// ```rust
/// use dagrs::{ DefaultTask, Output };
/// let mut task = DefaultTask::with_closure("task",|_input,_env|Output::empty());
/// ```
/// Use the `with_action` function to create a task and give it a name and execution logic.
/// The execution logic is given in the form of a concrete type that implements the [`Complex`] trait.
/// For an explanation of the [`Complex`] feature, please see the `action` module.
///
/// ```rust
/// use dagrs::{ DefaultTask, Complex, Output, Input, EnvVar };
/// use std::sync::Arc;
///
/// struct Act(u32);
///
/// impl Complex for Act{
///     fn run(&self, input: Input, env: Arc<EnvVar>) -> Output{
///         Output::new(self.0+10)
///     }
/// }
///
/// let mut task = DefaultTask::with_action("task",Act(20));
/// ```
///
/// A default implementation of the Task trait. In general, use it to define the tasks of dagrs.
#[derive(Clone)]
pub struct DefaultTask {
    /// id is the unique identifier of each task, it will be assigned by the global [`IDAllocator`]
    /// when creating a new task, you can find this task through this identifier.
    id: usize,
    /// The task's name.
    name: String,
    /// Id of the predecessor tasks.
    precursors: Vec<usize>,
    /// Perform specific actions.
    action: Action,
}

impl DefaultTask {
    /// Create a task and specify the task name. You may need to call the `set_action` or `set_closure` function later.
    pub fn new(name: &str) -> Self {
        let action = |_, _| Output::empty();
        DefaultTask {
            id: ID_ALLOCATOR.alloc(),
            action: Action::Closure(Arc::new(action)),
            name: name.to_owned(),
            precursors: Vec::new(),
        }
    }
    /// Create a task, give the task name, and provide a specific type that implements the [`Complex`] trait as the specific
    /// execution logic of the task.
    pub fn with_action(name: &str, action: impl Complex + Send + Sync + 'static) -> Self {
        Self::with_action_dyn(name, Arc::new(action))
    }

    /// Create a task, give the task name, and provide a dynamic task that implements the [`Complex`] trait as the specific
    /// execution logic of the task.
    pub fn with_action_dyn(name: &str, action: Arc<dyn Complex + Send + Sync>) -> Self {
        DefaultTask {
            id: ID_ALLOCATOR.alloc(),
            action: Action::Structure(action),
            name: name.to_owned(),
            precursors: Vec::new(),
        }
    }

    /// Create a task, give the task name, and provide a closure as the specific execution logic of the task.
    pub fn with_closure(
        name: &str,
        action: impl Fn(Input, Arc<EnvVar>) -> Output + Send + Sync + 'static,
    ) -> Self {
        Self::with_closure_dyn(name, Arc::new(action))
    }

    /// Create a task, give the task name, and provide a closure as the specific execution logic of the task.
    pub fn with_closure_dyn(
        name: &str,
        action: Arc<dyn Fn(Input, Arc<EnvVar>) -> Output + Send + Sync>,
    ) -> Self {
        DefaultTask {
            id: ID_ALLOCATOR.alloc(),
            action: Action::Closure(action),
            name: name.to_owned(),
            precursors: Vec::new(),
        }
    }

    /// Give the task a name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Tasks that shall be executed before this one.
    ///
    /// # Example
    /// ```rust
    /// use dagrs::{DefaultTask,Output};
    /// let t1 = DefaultTask::with_closure("Task 1", |_input,_env|Output::empty());
    /// let mut t2 = DefaultTask::with_closure("Task 2",|_input,_env|Output::empty());
    /// t2.set_predecessors(&[&t1]);
    /// ```
    /// In above code, `t1` will be executed before `t2`.
    pub fn set_predecessors<'a>(
        &mut self,
        predecessors: impl IntoIterator<Item = &'a &'a DefaultTask>,
    ) {
        self.precursors
            .extend(predecessors.into_iter().map(|t| t.id()))
    }

    /// The same as `exec_after`, but input are tasks' ids
    /// rather than reference to [`DefaultTask`].
    pub fn set_predecessors_by_id(&mut self, predecessors_id: impl IntoIterator<Item = usize>) {
        self.precursors.extend(predecessors_id)
    }

    /// Provide a closure to specify execution logic for the task.
    pub fn set_closure(
        &mut self,
        action: impl Fn(Input, Arc<EnvVar>) -> Output + Send + Sync + 'static,
    ) {
        self.action = Action::Closure(Arc::new(action));
    }

    /// Provide a concrete type that implements the [`Complex`] trait to specify execution logic for the task.
    pub fn set_action(&mut self, action: impl Complex + Send + Sync + 'static) {
        self.action = Action::Structure(Arc::new(action))
    }
}

impl Task for DefaultTask {
    fn action(&self) -> Action {
        self.action.clone()
    }

    fn precursors(&self) -> &[usize] {
        &self.precursors
    }

    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Default for DefaultTask {
    fn default() -> Self {
        let id = ID_ALLOCATOR.alloc();
        let name = format!("Task {}", id);
        let action = |_, _| Output::empty();
        Self {
            id,
            name,
            precursors: Vec::new(),
            action: Action::Closure(Arc::new(action)),
        }
    }
}
