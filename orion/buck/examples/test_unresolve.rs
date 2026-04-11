use api_model::buck2::types::ProjectRelativePath;
use td_util_buck::cells::CellInfo;

fn main() {
    // 模拟实际的 buck2 audit cell --json 输出
    let cells_json = r#"{
        "root": "/Users/jackie/work/project/buck2_test",
        "toolchains": "/Users/jackie/work/project/buck2_test/toolchains",
        "prelude": "/Users/jackie/work/project/buck2_test/prelude",
        "none": "/Users/jackie/work/project/buck2_test/none"
    }"#;
    
    println!("Parsing cells JSON...");
    let cells = match CellInfo::parse(cells_json) {
        Ok(c) => {
            println!("✓ Cells parsed successfully!");
            c
        }
        Err(e) => {
            println!("✗ Failed to parse cells: {}", e);
            return;
        }
    };
    
    // 测试 toolchains/BUCK 的解析
    let path = ProjectRelativePath::new("toolchains/BUCK");
    match cells.unresolve(&path) {
        Ok(cell_path) => {
            println!(
                "✓ Successfully resolved: {} -> {}",
                path.as_str(),
                cell_path.as_str()
            );
        }
        Err(e) => {
            println!("✗ Failed to resolve: {} - Error: {}", path.as_str(), e);
        }
    }

    // 测试根目录的 BUCK
    let path2 = ProjectRelativePath::new("BUCK");
    match cells.unresolve(&path2) {
        Ok(cell_path) => {
            println!(
                "✓ Successfully resolved: {} -> {}",
                path2.as_str(),
                cell_path.as_str()
            );
        }
        Err(e) => {
            println!("✗ Failed to resolve: {} - Error: {}", path2.as_str(), e);
        }
    }
    
    // 测试 toolchains 目录本身
    let path3 = ProjectRelativePath::new("toolchains");
    match cells.unresolve(&path3) {
        Ok(cell_path) => {
            println!(
                "✓ Successfully resolved: {} -> {}",
                path3.as_str(),
                cell_path.as_str()
            );
        }
        Err(e) => {
            println!("✗ Failed to resolve: {} - Error: {}", path3.as_str(), e);
        }
    }
}
