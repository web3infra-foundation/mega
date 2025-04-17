use std::sync::Arc;

use dagrs::EnvVar;

#[test]
fn env_set_get_test() {
    let env = init_env();
    assert_eq!(env.get::<usize>("test1"), Some(1usize));
    assert_eq!(env.get::<usize>("test2"), None);
    assert_eq!(env.get_ref::<String>("test3"), Some(&"3".to_string()))
}

#[test]
fn multi_thread_immutable_env_test() {
    let env = Arc::new(init_env());
    let mut handles = Vec::new();

    let env1 = env.clone();
    handles.push(std::thread::spawn(move || {
        assert_eq!(env1.get::<usize>("test1"), Some(1usize));
    }));

    let env2 = env.clone();
    handles.push(std::thread::spawn(move || {
        assert_eq!(env2.get::<usize>("test1"), Some(1usize));
    }));

    let env3 = env.clone();
    handles.push(std::thread::spawn(move || {
        assert_eq!(env3.get::<usize>("test2"), None);
    }));
    handles
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
}

fn init_env() -> EnvVar {
    let mut env = EnvVar::new();

    env.set("test1", 1usize);
    env.set("test2", 2i32);
    env.set("test3", "3".to_string());
    env
}
