//! Only use Dag, execute a job. The graph is as follows:
//!
//!    ↱----------↴
//!    B -→ E --→ G
//!  ↗    ↗     ↗
//! A --→ C    /
//!  ↘    ↘  /
//!   D -→ F
//!
//! The final execution result is 272.

extern crate dagrs;

use dagrs::{Complex, Dag, DefaultTask, EnvVar, Input, Output};
use std::sync::Arc;

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
    // initialization log.
    env_logger::init();

    // generate some tasks.
    let a = DefaultTask::with_action("Compute A", Compute(1));

    let mut b = DefaultTask::with_action("Compute B", Compute(2));

    let mut c = DefaultTask::new("Compute C");
    c.set_action(Compute(4));

    let mut d = DefaultTask::new("Compute D");
    d.set_action(Compute(8));

    let mut e = DefaultTask::with_closure("Compute E", |input, env| {
        let base = env.get::<usize>("base").unwrap();
        let mut sum = 16;
        input
            .get_iter()
            .for_each(|i| sum += i.get::<usize>().unwrap() * base);
        Output::new(sum)
    });
    let mut f = DefaultTask::with_closure("Compute F", |input, env| {
        let base = env.get::<usize>("base").unwrap();
        let mut sum = 32;
        input
            .get_iter()
            .for_each(|i| sum += i.get::<usize>().unwrap() * base);
        Output::new(sum)
    });

    let mut g = DefaultTask::new("Compute G");
    g.set_closure(|input, env| {
        let base = env.get::<usize>("base").unwrap();
        let mut sum = 64;
        input
            .get_iter()
            .for_each(|i| sum += i.get::<usize>().unwrap() * base);
        Output::new(sum)
    });

    // Set up task dependencies.
    b.set_predecessors(&[&a]);
    c.set_predecessors(&[&a]);
    d.set_predecessors(&[&a]);
    e.set_predecessors(&[&b, &c]);
    f.set_predecessors(&[&c, &d]);
    g.set_predecessors(&[&b, &e, &f]);
    // Create a new Dag.
    let mut dag = Dag::with_tasks(vec![a, b, c, d, e, f, g]);
    // Set a global environment variable for this dag.
    let mut env = EnvVar::new();
    env.set("base", 2usize);
    dag.set_env(env);
    // Start executing this dag
    assert!(dag.start().is_ok());
    // Get execution result.
    let res = dag.get_result::<usize>().unwrap();
    println!("The result is {}.", res);
}
