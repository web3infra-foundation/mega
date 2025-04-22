//! Use Engine to manage multiple Dag jobs.

extern crate dagrs;

use std::collections::HashMap;

use dagrs::{Dag, DefaultTask, Engine, Output};
fn main() {
    // initialization log.
    env_logger::init();
    // Create an Engine.
    let mut engine = Engine::default();

    // Create some task for dag1.
    let t1_a = DefaultTask::with_closure("Compute A1", |_, _| Output::new(20usize));
    let mut t1_b = DefaultTask::with_closure("Compute B1", |input, _| {
        let mut sum = 10;
        input.get_iter().for_each(|input| {
            sum += input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });
    let mut t1_c = DefaultTask::with_closure("Compute C1", |input, _| {
        let mut sum = 20;
        input.get_iter().for_each(|input| {
            sum += input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });

    let mut t1_d = DefaultTask::with_closure("Compute D1", |input, _| {
        let mut sum = 30;
        input.get_iter().for_each(|input| {
            sum += input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });
    t1_b.set_predecessors(&[&t1_a]);
    t1_c.set_predecessors(&[&t1_a]);
    t1_d.set_predecessors(&[&t1_b, &t1_c]);
    let dag1 = Dag::with_tasks(vec![t1_a, t1_b, t1_c, t1_d]);
    // Add dag1 to engine.
    engine.append_dag("graph1", dag1);

    // Create some task for dag2.
    let t2_a = DefaultTask::with_closure("Compute A2", |_, _| Output::new(2usize));
    let mut t2_b = DefaultTask::with_closure("Compute B2", |input, _| {
        let mut sum = 4;
        input.get_iter().for_each(|input| {
            sum *= input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });
    let mut t2_c = DefaultTask::with_closure("Compute C2", |input, _| {
        let mut sum = 8;
        input.get_iter().for_each(|input| {
            sum *= input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });
    let mut t2_d = DefaultTask::with_closure("Compute D2", |input, _| {
        let mut sum = 16;
        input.get_iter().for_each(|input| {
            sum *= input.get::<usize>().unwrap();
        });
        Output::new(sum)
    });
    t2_b.set_predecessors(&[&t2_a]);
    t2_c.set_predecessors(&[&t2_b]);
    t2_d.set_predecessors(&[&t2_c]);
    let dag2 = Dag::with_tasks(vec![t2_a, t2_b, t2_c, t2_d]);
    // Add dag2 to engine.
    engine.append_dag("graph2", dag2);
    // Read tasks from configuration files and resolve to dag3.
    let dag3 = Dag::with_yaml("tests/config/correct.yaml", HashMap::new()).unwrap();
    // Add dag3 to engine.
    engine.append_dag("graph3", dag3);
    // Execute dag in order, the order should be dag1, dag2, dag3.
    assert!(engine.run_sequential().is_ok());
    // Get the execution results of dag1 and dag2.
    assert_eq!(
        engine.get_dag_result::<usize>("graph1").unwrap().as_ref(),
        &100
    );
    assert_eq!(
        engine.get_dag_result::<usize>("graph2").unwrap().as_ref(),
        &1024
    );
}
