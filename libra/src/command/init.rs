// command/init.rs - init命令的所有业务逻辑都放在这里
use clap::Parser;
use git2::{Repository, RepositoryInitOptions};
use std::path::PathBuf;
use std::fs;

/// Init命令参数定义（适配--bare + --separate-git-dir）
#[derive(Debug, Parser)]
pub struct InitArgs {
    /// 创建裸仓库（默认true，兼容原有逻辑）
    #[arg(long, default_value_t = true, help = "创建裸仓库（无工作目录）")]
    bare: bool,

    /// 工作目录路径（非裸仓库时使用）
    #[arg(long, help = "工作目录路径")]
    workdir: Option<PathBuf>,

    /// 分离的git仓库目录路径（可选，核心参数）
    #[arg(long, help = "将版本控制数据存储在指定路径")]
    separate_git_dir: Option<PathBuf>,
}

/// 执行init命令的核心函数
pub fn run(args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 处理裸仓库模式
    if args.bare {
        // 确定git仓库目录
        let git_dir = match &args.separate_git_dir {
            Some(dir) => dir,
            None => return Err("For bare repository, --separate-git-dir must be specified".into()),
        };

        // 确保目标目录的父目录存在
        if let Some(parent) = git_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        // 初始化裸仓库
        match Repository::init_bare(git_dir) {
            Ok(_repo) => {
                println!(
                    "[SUCCESS] Initialized bare libra repository at: {:?}",
                    git_dir
                );
            }
            Err(e) => {
                return Err(format!("Failed to initialize bare repository: {}", e).into());
            }
        }
    } 
    // 2. 处理非裸仓库模式
    else {
        // 确定工作目录
        let workdir = match &args.workdir {
            Some(dir) => dir,
            None => return Err("For non-bare repository, --workdir must be specified".into()),
        };

        // 确保工作目录存在
        fs::create_dir_all(workdir)?;

        // 根据是否指定--separate-git-dir选择初始化方式
        match &args.separate_git_dir {
            // 使用分离的git目录
            Some(git_dir) => {
                // 确保git目录的父目录存在
                if let Some(parent) = git_dir.parent() {
                    fs::create_dir_all(parent)?;
                }

                // 初始化带有分离git目录的仓库
                match Repository::init_ext(workdir, RepositoryInitOptions::new()
                    .separate_git_dir(git_dir)
                    .bare(false)) {
                    Ok(_repo) => {
                        println!(
                            "[SUCCESS] Initialized libra repository with working directory at: {:?} and separate git directory at: {:?}",
                            workdir, git_dir
                        );
                    }
                    Err(e) => {
                        return Err(format!("Failed to initialize repository with separate git directory: {}", e).into());
                    }
                }
            },
            // 使用默认的.git目录
            None => {
                match Repository::init(workdir) {
                    Ok(_repo) => {
                        println!(
                            "[SUCCESS] Initialized libra repository at: {:?}",
                            workdir
                        );
                    }
                    Err(e) => {
                        return Err(format!("Failed to initialize repository: {}", e).into());
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_bare_with_separate_git_dir() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();
        let git_dir = temp_dir.path().join("test-repo.git");

        // 构造参数
        let args = InitArgs {
            bare: true,
            workdir: None,
            separate_git_dir: Some(git_dir.clone()),
        };

        // 执行命令
        let result = run(&args);

        // 验证结果
        assert!(result.is_ok());
        assert!(git_dir.exists());
        assert!(git_dir.join("HEAD").exists());
    }

    #[test]
    fn test_init_non_bare_with_separate_git_dir() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();
        let workdir = temp_dir.path().join("workdir");
        let git_dir = temp_dir.path().join("git-dir");

        // 构造参数
        let args = InitArgs {
            bare: false,
            workdir: Some(workdir.clone()),
            separate_git_dir: Some(git_dir.clone()),
        };

        // 执行命令
        let result = run(&args);

        // 验证结果
        assert!(result.is_ok());
        assert!(workdir.exists());
        assert!(git_dir.exists());
        assert!(git_dir.join("HEAD").exists());
        assert!(workdir.join(".git").exists());
    }

    #[test]
    fn test_init_non_bare_without_separate_git_dir() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();
        let workdir = temp_dir.path().join("workdir");

        // 构造参数
        let args = InitArgs {
            bare: false,
            workdir: Some(workdir.clone()),
            separate_git_dir: None,
        };

        // 执行命令
        let result = run(&args);

        // 验证结果
        assert!(result.is_ok());
        assert!(workdir.exists());
        assert!(workdir.join(".git").exists());
    }

    #[test]
    fn test_init_bare_without_separate_git_dir() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 构造参数
        let args = InitArgs {
            bare: true,
            workdir: None,
            separate_git_dir: None,
        };

        // 执行命令
        let result = run(&args);

        // 验证结果
        assert!(result.is_err());
    }
}