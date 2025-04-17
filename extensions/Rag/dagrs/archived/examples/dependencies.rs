use dagrs::{dependencies, Complex, EnvVar, Input, Output};
use std::sync::Arc;

/// The `dependencies` macro allows users to specify all task dependencies in an easy-to-understand
/// way. It will return to the user a series of `DefaultTask` in the order of tasks given by the user.
///
/// # Example
///
///    ↱----------↴
///    B -→ E --→ G
///  ↗    ↗     ↗
/// A --→ C    /
///  ↘    ↘  /
///   D -→ F
///
/// If you want to define a task graph with such dependencies, the code is as follows:
///
/// let mut tasks=dependencies!(
///     a -> b c d,
///     b -> e g,
///     c -> e f,
///     d -> f,
///     e -> g,
///     f -> g,
///     g ->
/// );
///
/// Note that although task g has no successor tasks, "g->" must also be written. The return
/// value type tasks is a Vec<DefaultTask>. The name of each task is the same as the given
/// identifier, which can be expressed as an array as [ "a","b","c","d","e","f","g"].

struct Compute(usize);

impl Complex for Compute {
    fn run(&self, input: Input, env: Arc<EnvVar>) -> Output {
        let base = env.get::<usize>("base").unwrap();
        let mut sum = self.0;
        input
            .get_iter()
            .for_each(|i| sum += i.get::<usize>().unwrap() * base);
        Output::new(sum)
    }
}

fn main() {
    env_logger::init();
    let mut tasks = dependencies!(
            a -> b c d,
            b -> e g,
            c -> e f,
            d -> f,
            e -> g,
            f -> g,
            g ->
    );
    let mut x = 1;
    for task in tasks.iter_mut().take(4) {
        task.set_action(Compute(x * 2));
        x *= 2;
    }

    for task in tasks.iter_mut().skip(4) {
        task.set_closure(|input, env| {
            let base = env.get::<usize>("base").unwrap();
            let mut sum = 0;
            input
                .get_iter()
                .for_each(|i| sum += i.get::<usize>().unwrap() * base);
            Output::new(sum)
        });
    }

    let mut dag = dagrs::Dag::with_tasks(tasks);
    let mut env = EnvVar::new();
    env.set("base", 2usize);
    dag.set_env(env);
    assert!(dag.start().is_ok());
}
