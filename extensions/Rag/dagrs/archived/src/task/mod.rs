//! Relevant definitions of tasks.
//!
//! # [`Task`]: the basic unit of scheduling
//!
//! A [`Task`] is the basic unit for scheduling execution of a dagrs. [`Task`] itself is a trait and users
//! should use its concrete implementation [`DefaultTask`]. Of course, users can also customize [`Task`],
//! but it should be noted that whether it is the default [`DefaultTask`] or a user-defined task type, they
//! all need to have the following fields:
//! - `id`: type is `usize`. When allocating tasks, there is a global task `id` allocator.
//!   Users can call the `alloc_id()` function to assign ids to tasks, and the obtained `id` type is `usize`.
//! - `name`: type is `String`. This field represents the task name.
//! - `action`: type is [`Action`]. This field is used to store the specific execution logic of the task.
//! - `precursors`: type is `Vec<usize>`. This field is used to store the predecessor task `id` of this task.
//!
//! # [`Action`]: specific logical behavior
//!
//! Each task has an [`Action`] field inside, which stores the specific execution logic of the task.
//! [`Action`] is an enumeration type. For [`Simple`] execution logic, you only need to provide a closure for [`Action`].
//! For slightly more complex execution logic, you can implement the [`Complex`] trait. For detailed analysis,
//! please see the `action` module.
//!
//! # [`Input`] and [`Output`]
//!
//! Each task may produce output and may require the output of its predecessor task as its input.
//! [`Output`] is used to construct and store the output obtained by task execution. [`Input`] is used as a tool
//! to provide users with the output of the predecessor task.
use std::fmt::Debug;
use std::sync::atomic::AtomicUsize;

pub use self::action::{Action, Complex, Simple};
pub use self::cmd::CommandAction;
pub use self::default_task::DefaultTask;
pub use self::state::Content;
pub(crate) use self::state::ExecState;
pub use self::state::{Input, Output};

mod action;
mod cmd;
mod default_task;
mod state;
/// The Task trait
///
/// Tasks can have many attributes, among which `id`, `name`, `predecessor_tasks`, and
/// `action` attributes are required, and users can also customize some other attributes.
/// [`DefaultTask`] in this module is a [`Task`], the DAG engine uses it as the basic
/// task by default.
///
/// A task must provide methods to obtain precursors and required attributes, just as
/// the methods defined below, users who want to customize tasks must implement these methods.
pub trait Task: Send + Sync {
    /// Get a reference to an executable action.
    fn action(&self) -> Action;
    /// Get the id of all predecessor tasks of this task.
    fn precursors(&self) -> &[usize];
    /// Get the id of this task.
    fn id(&self) -> usize;
    /// Get the name of this task.
    fn name(&self) -> &str;
}

/// IDAllocator for DefaultTask
struct IDAllocator {
    id: AtomicUsize,
}

impl Debug for dyn Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{},\t{},\t{:?}",
            self.id(),
            self.name(),
            self.precursors()
        )
    }
}

impl IDAllocator {
    fn alloc(&self) -> usize {
        let origin = self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if origin > self.id.load(std::sync::atomic::Ordering::Relaxed) {
            panic!("Too many tasks.")
        } else {
            origin
        }
    }
}

/// The global task uniquely identifies an instance of the allocator.
static ID_ALLOCATOR: IDAllocator = IDAllocator {
    id: AtomicUsize::new(1),
};

/// public function to assign task's id.
pub fn alloc_id() -> usize {
    ID_ALLOCATOR.alloc()
}
