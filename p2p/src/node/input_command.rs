use std::sync::Arc;

use libp2p::Swarm;
use tokio::sync::Mutex;

use crate::network::behaviour;
use crate::node::command_handler::CmdHandler;
use crate::node::ClientParas;

pub async fn handle_input_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    line: String,
) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let mut args = line.split_whitespace();
    match args.next() {
        Some("kad") => {
            handle_kad_command(swarm, client_paras, args.collect()).await;
        }
        Some("mega") => {
            handle_mega_command(swarm, client_paras, args.collect()).await;
        }
        Some("nostr") => {
            handle_nostr_command(swarm, client_paras, args.collect()).await;
        }
        _ => {
            eprintln!("expected command: kad, mega, nostr");
        }
    }
}

pub async fn handle_kad_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    args: Vec<&str>,
) {
    let cmd_handler = CmdHandler {
        swarm,
        client_paras,
    };
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        Some("get") => {
            let key = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            cmd_handler.kad_get(key).await;
        }
        Some("put") => {
            let key = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            let value = {
                match args_iter.next() {
                    Some(vaule) => vaule,
                    None => {
                        eprintln!("Expected value");
                        return;
                    }
                }
            };
            cmd_handler.kad_put(key, value).await
        }
        Some("k_buckets") => cmd_handler.k_buckets().await,
        Some("get_peer") => {
            cmd_handler.get_peer(args_iter.next()).await;
        }
        _ => {
            eprintln!("expected command: get, put, k_buckets, get_peer");
        }
    }
}

pub async fn handle_mega_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    args: Vec<&str>,
) {
    let cmd_handler = CmdHandler {
        swarm,
        client_paras,
    };
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        //mega provide ${your_repo}.git
        Some("provide") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.provide(&repo_name).await;
        }
        Some("search") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.search(&repo_name).await;
        }
        Some("clone") => {
            // mega clone p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
            let mega_address = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected mega_address");
                        return;
                    }
                }
            };
            cmd_handler.clone(mega_address).await;
        }
        Some("pull") => {
            // mega pull p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
            let mega_address = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected mega_address");
                        return;
                    }
                }
            };
            cmd_handler.pull(mega_address).await;
        }
        Some("clone-object") => {
            // mega clone-object mega_test.git
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.clone_obj(&repo_name).await;
        }
        Some("pull-object") => {
            // mega pull-object mega_test.git
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.pull_obj(&repo_name).await;
        }
        _ => {
            eprintln!("expected command: clone, pull, provide, clone-object, pull-object");
        }
    }
}

pub async fn handle_nostr_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    args: Vec<&str>,
) {
    let cmd_handler = CmdHandler {
        swarm,
        client_paras,
    };
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        Some("subscribe") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.subscribe(&repo_name).await;
        }
        Some("event-update") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_update(&repo_name).await;
        }
        Some("event-merge") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_merge(&repo_name).await;
        }
        Some("event-issue") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_issue(&repo_name).await;
        }
        _ => {
            eprintln!("expected command: subscribe, event-update, event-issue");
        }
    }
}
