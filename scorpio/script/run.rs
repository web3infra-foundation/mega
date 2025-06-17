use std::env;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn collect_files_recursively(dir: &Path) -> Vec<PathBuf> {
    if dir.file_name() == Some(std::ffi::OsStr::new(".git")) {
        return Vec::new(); // 忽略 .git 目录
    }
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            println!("path {:?} {:?}", path, path.is_file());

            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(collect_files_recursively(&path));
            }
        }
    }
    files
}

fn main() {
    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("用法: {} <目录路径>", args[0]);
        return;
    }

    let target_dir = Path::new(&args[1]);
    if !target_dir.exists() {
        eprintln!("错误：指定路径不存在");
        return;
    }
    if !target_dir.is_dir() {
        eprintln!("错误：指定路径不是目录");
        return;
    }

    // 切换到目标目录
    let start_cd = Instant::now();

    if let Err(e) = env::set_current_dir(target_dir) {
        eprintln!("错误：无法切换到目录 {}: {}", target_dir.display(), e);
        return;
    }
    let duration_cd = start_cd.elapsed();

    println!("已切换到目录: {}", target_dir.display());
    
    // 获取当前工作目录（应该是我们刚切换到的目录）
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("错误：无法获取当前目录: {}", e);
            return;
        }
    };
    
    println!("当前工作目录: {}", current_dir.display());

    // 从当前目录开始递归收集文件（使用相对路径 "."）
    let files = collect_files_recursively(Path::new("."));
    
    println!("共找到 {} 个文件", files.len());
    for file in &files {
        println!("{}", file.display());
    }
    
    // 测试 stat 时间
    let start_stat = Instant::now();
    let _stats: Vec<Option<Metadata>> = files
        .iter()
        .map(|f| fs::metadata(f).ok())
        .collect();
    let duration_stat = start_stat.elapsed();

    // 测试文件读取时间
    let start_read = Instant::now();
    let mut total_bytes = 0u64;
    for f in &files {
        if let Ok(mut file) = File::open(f) {
            let mut buffer = Vec::new();
            if let Ok(bytes_read) = file.read_to_end(&mut buffer) {
                total_bytes += bytes_read as u64;
            }
        }
    }
    let duration_read = start_read.elapsed();

    // 输出统计信息
    println!("\n===== 性能统计 =====");
    println!("文件数量: {}", files.len());
    println!("cd 加载目录: {:.3?}", duration_cd);

    println!("总读取字节数: {:.2} MB", total_bytes as f64 / (1024.0 * 1024.0));
    println!("Stat 操作时间: {:.3?}", duration_stat);
    println!("文件读取时间: {:.3?}", duration_read);
    
    if duration_stat.as_secs_f64() > 0.0 {
        println!("Stat 操作速率: {:.2} 文件/秒", files.len() as f64 / duration_stat.as_secs_f64());
    }
    
    if duration_read.as_secs_f64() > 0.0 {
        let throughput = (total_bytes as f64 / (1024.0 * 1024.0)) / duration_read.as_secs_f64();
        println!("读取吞吐量: {:.2} MB/秒", throughput);
    }
}