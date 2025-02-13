## Mercury Module - Git Internal Module

### Performance

> [!TIP]
> Here are some performance tips that you can use to significantly improve performance when using `Mercury` crates as a dependency.

In certain versions of Rust, using `HashMap` on Windows can lead to performance issues. This is due to the allocation strategy of the internal heap memory allocator. To mitigate these performance issues on Windows, you can use [mimalloc](https://github.com/microsoft/mimalloc). (See [this issue](https://github.com/rust-lang/rust/issues/121747) for more details.)

On other platforms, you can also experiment with [jemalloc](https://github.com/jemalloc/jemalloc) or [mimalloc](https://github.com/microsoft/mimalloc) to potentially improve performance.

A simple approach:

1. Change Cargo.toml to use mimalloc on Windows and jemalloc on other platforms.

   ```toml
   [target.'cfg(not(windows))'.dependencies]
   jemallocator = "0.5.4"
   
   [target.'cfg(windows)'.dependencies]
   mimalloc = "0.1.43"
   ```

2. Add `#[global_allocator]` to the main.rs file of the program to specify the allocator.

   ```rust
   #[cfg(not(target_os = "windows"))]
   #[global_allocator]
   static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
   
   #[cfg(target_os = "windows")]
   #[global_allocator]
   static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
   ```

   