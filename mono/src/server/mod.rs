use clap::Args;

pub mod http_server;
pub mod ssh_server;
pub mod trace_context;

pub use trace_context::{TraceContext, trace_id};

#[derive(Args, Clone, Debug)]
pub struct CommonHttpOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(short = 'p', long, default_value_t = 8000)]
    pub port: u16,
}
