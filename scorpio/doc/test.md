## 调试 `test_antares_mount` 流程（no-run → LLDB）

1. **编译但不运行指定测试**

	```bash
	cargo test --lib antares::fuse::tests::test_antares_mount --no-run
	```

	- Cargo 只会编译库测试，终端输出中会提示实际生成的测试可执行文件，例如 `target/debug/deps/scorpio-020a49fada48c0ff`。
sudo umount /tmp/antares_test_mount/mnt 
sudo rm -rf /tmp/antares_test_mount
2. **启动 LLDB 并加载测试可执行文件**

	```bash
	sudo lldb --   ../target/debug/deps/scorpio-020a49fada48c0ff   --exact antares::fuse::tests::test_antares_mount --ignored --nocapture
	```

	- 由于测试需要 FUSE/root 权限，这里用 `sudo` 启动 LLDB。

3. **在 LLDB 中设置环境变量与测试参数（可选但常用）**

	```
	(lldb) settings set target.env-vars RUST_LOG=rfuse3=trace
	(lldb) settings set target.run-args --exact antares::fuse::tests::test_antares_mount --ignored --nocapture
	```

	- 第一条开启 rfuse3 trace 日志，方便观察 FUSE opcode。
	- 第二条告诉测试运行器：仅运行 `test_antares_mount`，包含 `#[ignore]` 的用例，并打印 stdout/stderr。

4. **运行与调试**

	```
	(lldb) run
	```

	- 需要重跑时，可调整 `run-args` 或环境变量后再次 `run`。
	- 调试过程中可用 `breakpoint set ...`、`bt`、`frame variable` 等 LLDB 命令定位问题。

5. **退出**

	- 测试结束后 `process exit`，如需退出 LLDB 使用 `quit`。

> 注意：若重新编译导致测试可执行文件的哈希变化，请回到步骤 2 更新路径。
