// libra/src/command/init.rs
use clap::Parser;
use git2::Repository;
use std::fs;
use std::path::PathBuf;

/// Init命令参数结构（新增--separate-git-dir参数）
#[derive(Debug, Parser)]
pub struct InitArgs {
    /// 创建裸仓库（兼容原有逻辑）
    #[arg(long, default_value_t = true, help = "创建裸仓库（无工作目录）")]
    bare: bool,

    /// 分离的git仓库目录路径（必填，核心参数）
    #[arg(long, required = true, help = "将版本控制数据存储在指定路径")]
    separate_git_dir: PathBuf,
}

/// 执行init命令的核心函数
pub fn run(args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 确保目标目录的父目录存在（自动创建不存在的路径）
    if let Some(parent) = args.separate_git_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    // 2. 处理裸仓库初始化（兼容--bare参数）
    if args.bare {
        // 初始化git裸仓库
        match Repository::init_bare(&args.separate_git_dir) {
            Ok(_repo) => {
                println!(
                    "[SUCCESS] Initialized bare libra repository at: {:?}",
                    args.separate_git_dir
                );
            }
            Err(e) => {
                return Err(format!("Failed to initialize bare repository: {}", e).into());
            }
        }
    } else {
        // 非裸仓库提示（暂不支持，保留扩展空间）
        return Err("Non-bare repository is not supported yet".into());
    }

    // 函数正常返回
    Ok(())
}

// 3个单元测试用例（满足Issue要求）
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // 测试1：正常传入--separate-git-dir参数
    #[test]
    fn test_separate_git_dir_normal() {
        let temp_git = TempDir::new().unwrap();
        let args = InitArgs {
            bare: true,
            separate_git_dir: temp_git.path().to_path_buf(),
        };
        assert!(run(&args).is_ok());
        assert!(temp_git.path().join("config").exists());
    }

    // 测试2：指定路径不存在，自动创建
    #[test]
    fn test_separate_git_dir_auto_create() {
        let temp_root = TempDir::new().unwrap();
        let non_exist_git = temp_root.path().join("a/b/c/test-repo");
        let args = InitArgs {
            bare: true,
            separate_git_dir: non_exist_git.clone(),
        };
        assert!(run(&args).is_ok());
        assert!(non_exist_git.exists());
    }

    // 测试3：--bare + --separate-git-dir 兼容
    #[test]
    fn test_bare_with_separate_git_dir() {
        let temp_git = TempDir::new().unwrap();
        let args = InitArgs {
            bare: true,
            separate_git_dir: temp_git.path().to_path_buf(),
        };
        assert!(run(&args).is_ok());
    }
}