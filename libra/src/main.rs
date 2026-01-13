use clap::{Parser, Subcommand};
use command::init::{InitArgs, run};

// 定义所有子命令（当前只有init）
#[derive(Debug, Subcommand)]
enum Commands {
    Init(InitArgs), // 绑定init子命令和其参数
}

// 定义程序的主参数结构
#[derive(Debug, Parser)]
#[command(name = "libra", about = "Libra project initialization tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands, // 接收子命令
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 匹配子命令并执行对应逻辑
    match cli.command {
        Commands::Init(args) => run(&args),
    }
}

// 导入command模块（确保init子模块能被找到）
mod command;