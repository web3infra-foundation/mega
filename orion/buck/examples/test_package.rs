use td_util_buck::types::{CellPath, Package};

fn main() {
    // 测试 toolchains cell 的 package
    let toolchains_package = Package::new("toolchains//");
    println!("Package: {}", toolchains_package.as_str());
    println!(
        "As CellPath: {}",
        toolchains_package.as_cell_path().as_str()
    );

    // 测试 toolchains/BUCK 的 CellPath
    let buck_path = CellPath::new("toolchains//BUCK");
    println!("\nBUCK path: {}", buck_path.as_str());

    // 测试 strip_prefix
    let package_str = toolchains_package.as_str();
    let path_str = buck_path.as_str();

    println!("\nTesting strip_prefix:");
    println!("  package_str: '{}'", package_str);
    println!("  path_str: '{}'", path_str);

    if let Some(relative) = path_str.strip_prefix(package_str) {
        println!("  ✓ Stripped: '{}'", relative);
        let relative = relative.strip_prefix('/').unwrap_or(relative);
        println!("  After removing leading '/': '{}'", relative);
        println!("  Contains '/': {}", relative.contains('/'));
        println!("  Is 'BUCK': {}", relative == "BUCK");
    } else {
        println!("  ✗ strip_prefix failed");
    }
}
