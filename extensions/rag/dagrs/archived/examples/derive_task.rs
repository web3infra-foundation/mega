use dagrs::{Action, CustomTask, Output, Task};
use std::sync::Arc;

/// `CustomTask` is a derived macro that may be used when customizing tasks. It can only be
/// marked on the structure, and the user needs to specify four attributes of the custom task
/// type, which are task(attr="id"), task(attr = "name"), task(attr = "precursors ") and
/// task(attr = "action"), which are used in the `derive_task` example.
///
/// # Example
///
/// ```rust
/// #[derive(CustomTask)]
/// struct MyTask {
///     #[task(attr = "id")]
///     id: usize,
///     #[task(attr = "name")]
///     name: String,
///     #[task(attr = "precursors")]
///     pre: Vec<usize>,
///     #[task(attr = "action")]
///     action: Action,
/// }
/// ```
#[derive(CustomTask)]
struct MyTask {
    #[task(attr = "id")]
    id: usize,
    #[task(attr = "name")]
    name: String,
    #[task(attr = "precursors")]
    pre: Vec<usize>,
    #[task(attr = "action")]
    action: Action,
}

fn main() {
    let action = Action::Closure(Arc::new(|_, _| Output::empty()));
    let task = MyTask {
        id: 10,
        name: "mytask".to_owned(),
        pre: vec![1, 2],
        action,
    };
    println!("{}\t{}\t{:?}", task.id(), task.name(), task.precursors());
}
