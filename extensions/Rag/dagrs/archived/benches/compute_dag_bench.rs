use criterion::{criterion_group, criterion_main, Criterion};

use dagrs::{Dag, DefaultTask, EnvVar, Input, Output, Task};
use std::sync::Arc;

fn calc(input: Input, env: Arc<EnvVar>) -> Output {
    let base = env.get::<usize>("base").unwrap();
    let mut sum = 2;
    input.get_iter().for_each(|i| {
        let mut i_usize = *i.get::<usize>().unwrap();
        // avoid overflow
        if i_usize > 1e6 as usize {
            i_usize = 2;
        }

        sum += i_usize * base
    });
    Output::new(sum)
}

fn compute_dag(tasks: Vec<DefaultTask>) {
    let mut dag = Dag::with_tasks(tasks);
    let mut env = EnvVar::new();
    env.set("base", 2usize);
    dag.set_env(env);

    assert!(dag.start().is_ok());
    // Get execution result.
    let _res = dag.get_result::<usize>().unwrap();
}

fn compute_dag_bench(bencher: &mut Criterion) {
    env_logger::init();

    let mut tasks = (0..50usize)
        .map(|i_task| DefaultTask::with_closure(&i_task.to_string(), calc))
        .collect::<Vec<_>>();

    // consider 8 dependency for each task (except first 20 tasks)
    for i_task in 20..tasks.len() {
        let predecessors_id = ((i_task - 8)..i_task)
            .map(|i_dep| tasks[i_dep].id())
            .collect::<Vec<_>>();

        tasks[i_task].set_predecessors_by_id(predecessors_id);
    }

    bencher.bench_function("compute dag", |b| b.iter(|| compute_dag(tasks.clone())));
}

criterion_group!(
  name = benches;
  config = {

    #[allow(unused_mut)]
    let mut criterion = Criterion::default().sample_size(4000).noise_threshold(0.05);

    #[cfg(feature = "bench-prost-codec")]
    {
      use pprof::criterion::{PProfProfiler, Output::Protobuf};

      criterion = criterion.with_profiler(PProfProfiler::new(4000, Protobuf));
    }

    criterion
  };
  targets = compute_dag_bench
);

criterion_main!(benches);
