// build.rs - 编译时链接Windows系统库，解决libgit2-sys链接错误
fn main() {
    // 手动链接需要的Windows系统库，补全缺失的外部符号
    println!("cargo:rustc-link-lib=advapi32");  // 注册表相关API
    println!("cargo:rustc-link-lib=crypt32");   // 加密相关API
    println!("cargo:rustc-link-lib=ws2_32");    // 网络相关API
    println!("cargo:rustc-link-lib=user32");    // 窗口相关API
}