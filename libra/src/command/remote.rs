use crate::internal::config::Config;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum RemoteCmds {
    /// Add a remote
    Add {
        /// The name of the remote
        name: String,
        /// The URL of the remote
        url: String,
    },
    /// Remove a remote
    Remove {
        /// The name of the remote
        name: String,
    },
    /// List remotes
    #[command(name = "-v")]
    List,
    /// Show current remote repository
    Show,
}

pub async fn execute(command: RemoteCmds) {
    match command {
        RemoteCmds::Add { name, url } => {
            Config::insert("remote", Some(&name), "url", &url).await;
        }
        RemoteCmds::Remove { name } => {
            if let Err(e) = Config::remove_remote(&name).await {
                eprintln!("{}", e);
            }
        }
        RemoteCmds::List => {
            let remotes = Config::all_remote_configs().await;
            for remote in remotes {
                show_remote_verbose(&remote.name).await;
            }
        }
        RemoteCmds::Show => {
            let remotes = Config::all_remote_configs().await;
            for remote in remotes {
                println!("{}", remote.name);
            }
        }
    }
}

async fn show_remote_verbose(remote: &str) {
    // There can be multiple URLs for a remote, like Gitee & GitHub
    let urls = Config::get_all("remote", Some(remote), "url").await;
    match urls.first() {
        Some(url) => {
            println!("{} {} (fetch)", remote, url);
        }
        None => {
            eprintln!("fatal: no URL configured for remote '{}'", remote);
        }
    }
    for url in urls {
        println!("{} {} (push)", remote, url);
    }
}
