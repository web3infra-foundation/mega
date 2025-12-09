use std::path::PathBuf;

use clap::{Parser, Subcommand};
use scorpio::antares::{AntaresManager, AntaresPaths};
use scorpio::util::config;

/// Antares build overlay manager (skeleton).
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the configuration file (scorpio config).
    #[arg(long, default_value = "scorpio.toml")]
    config_path: String,
    /// Root path to place per-job upper layers (overrides config when set).
    #[arg(long)]
    upper_root: Option<PathBuf>,
    /// Root path to place per-job CL layers (overrides config when set).
    #[arg(long)]
    cl_root: Option<PathBuf>,
    /// Root path for per-job mountpoints (overrides config when set).
    #[arg(long)]
    mount_root: Option<PathBuf>,
    /// Path to persist mount state as TOML (overrides config when set).
    #[arg(long)]
    state_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Mount a new Antares job instance.
    Mount {
        /// Unique job identifier.
        job_id: String,
        /// Optional CL layer name; when set, creates a CL passthrough layer placeholder.
        #[arg(long)]
        cl: Option<String>,
    },
    /// Unmount a job instance.
    Umount {
        /// Job identifier to remove.
        job_id: String,
    },
    /// List tracked instances.
    List,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = config::init_config(&cli.config_path) {
        eprintln!("Failed to load config: {e}");
        std::process::exit(1);
    }

    let mut paths = AntaresPaths::from_global_config();
    if let Some(p) = cli.upper_root {
        paths.upper_root = p;
    }
    if let Some(p) = cli.cl_root {
        paths.cl_root = p;
    }
    if let Some(p) = cli.mount_root {
        paths.mount_root = p;
    }
    if let Some(p) = cli.state_file {
        paths.state_file = p;
    }

    let manager = AntaresManager::new(paths).await;

    match cli.command {
        Commands::Mount { job_id, cl } => match manager.mount_job(&job_id, cl.as_deref()).await {
            Ok(instance) => {
                println!(
                    "mounted job {} at {}",
                    job_id,
                    instance.mountpoint.display()
                );
            }
            Err(err) => {
                eprintln!("failed to mount job {}: {}", job_id, err);
                std::process::exit(1);
            }
        },
        Commands::Umount { job_id } => match manager.umount_job(&job_id).await {
            Ok(Some(_)) => println!("unmounted job {}", job_id),
            Ok(None) => {
                eprintln!("job {} not found", job_id);
                std::process::exit(1);
            }
            Err(err) => {
                eprintln!("failed to unmount job {}: {}", job_id, err);
                std::process::exit(1);
            }
        },
        Commands::List => {
            let items = manager.list().await;
            if items.is_empty() {
                println!("no active jobs");
            } else {
                for it in items {
                    let cl = it
                        .cl_dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    println!(
                        "job_id={} mount={} upper={} cl={}",
                        it.job_id,
                        it.mountpoint.display(),
                        it.upper_dir.display(),
                        cl
                    );
                }
            }
        }
    }
}
