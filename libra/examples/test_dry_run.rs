// 验证 rm --dry-run 功能的简单测试程序
use libra::command::remove::{execute, RemoveArgs};
use std::fs;
use std::io::Write;

fn main() {
    println!("正在测试 libra rm --dry-run 功能...");
    
    // 创建测试文件
    fs::create_dir_all("test_files").unwrap();
    let mut file1 = fs::File::create("test_files/file1.txt").unwrap();
    file1.write_all(b"Test content 1").unwrap();
    
    let mut file2 = fs::File::create("test_files/file2.txt").unwrap();
    file2.write_all(b"Test content 2").unwrap();
    
    println!("创建了测试文件: test_files/file1.txt, test_files/file2.txt");
    
    // 测试 dry-run 功能
    let args = RemoveArgs {
        pathspec: vec!["test_files/file1.txt".to_string(), "test_files/file2.txt".to_string()],
        cached: false,
        recursive: false,
        force: true, // 使用 force 模式避免需要 git 仓库
        dry_run: true,
    };
    
    println!("\n执行: libra rm --dry-run --force test_files/file1.txt test_files/file2.txt");
    
    match execute(args) {
        Ok(_) => {
            println!("✓ dry-run 执行成功！");
            
            // 验证文件仍然存在
            if fs::metadata("test_files/file1.txt").is_ok() && fs::metadata("test_files/file2.txt").is_ok() {
                println!(" 文件在 dry-run 后仍然存在（正确行为）");
            } else {
                println!(" 错误：dry-run 不应该实际删除文件");
            }
        }
        Err(e) => {
            println!("✗ dry-run 执行失败: {:?}", e);
        }
    }
    
    // 清理测试文件
    let _ = fs::remove_dir_all("test_files");
    println!("\n测试完成，已清理测试文件");
}
