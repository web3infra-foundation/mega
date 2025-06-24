```sh
#生成测试文件和目录,生成time_和时间相关的目录，/tmp/time_
script/run.sh

#使用rust代码进行访问文件和目录
rustc script/run.rs
#测试系统文件系统访问速度
script/run /tmp/time_
```

普通文件系统测试结果：

```sh
(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /tmp/test_202506162053/1/2/3
已切换到目录: /tmp/test_202506162053/1/2/3
当前工作目录: /tmp/test_202506162053/1/2/3
path "./1M_1_3.bin" true
path "./4" false
path "./4/1M_1_4.bin" true
path "./4/5" false
path "./4/5/1M_1_5.bin" true
path "./4/5/6" false
path "./4/5/6/1M_1_6.bin" true
共找到 4 个文件
./1M_1_3.bin
./4/1M_1_4.bin
./4/5/1M_1_5.bin
./4/5/6/1M_1_6.bin

===== 性能统计 =====
文件数量: 4
cd 加载目录: 2.815µs
总读取字节数: 4.00 MB
Stat 操作时间: 9.318µs
文件读取时间: 2.259ms
Stat 操作速率: 429276.67 文件/秒
读取吞吐量: 1770.36 MB/秒

(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /tmp/test_202506162053/2/2/3
已切换到目录: /tmp/test_202506162053/2/2/3
当前工作目录: /tmp/test_202506162053/2/2/3
path "./4" false
path "./4/5" false
path "./4/5/1M_2_5.bin" true
path "./4/5/6" false
path "./4/5/6/1M_2_6.bin" true
path "./4/1M_2_4.bin" true
path "./1M_2_3.bin" true
共找到 4 个文件
./4/5/1M_2_5.bin
./4/5/6/1M_2_6.bin
./4/1M_2_4.bin
./1M_2_3.bin

===== 性能统计 =====
文件数量: 4
cd 加载目录: 4.789µs
总读取字节数: 4.00 MB
Stat 操作时间: 7.734µs
文件读取时间: 1.836ms
Stat 操作速率: 517196.79 文件/秒
读取吞吐量: 2178.56 MB/秒
```

fuse文件系统性能测试：

```sh
#需要清空scorpio.toml里面的store文件夹的内容
#建议不要更改scorpio.toml里面的load_dir_depth参数，否则要修改创建文件夹深度和访问深度
#由于控制了加载深度，所以下面的测试不会访问到./4/5/6/1M_2_6.bin
#启动mono服务，推送/tmp/test_time_文件夹到mono
#scorpio下 cargo run 
#测试首次加载的性能
script/run /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
script/run /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3

(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
已切换到目录: /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
当前工作目录: /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
path "./4" false
path "./4/5" false
path "./4/5/6" false
path "./4/5/1M_1_5.bin" true
path "./4/1M_1_4.bin" true
path "./1M_1_3.bin" true
共找到 3 个文件
./4/5/1M_1_5.bin
./4/1M_1_4.bin
./1M_1_3.bin

===== 性能统计 =====
文件数量: 3
cd 加载目录: 299.299ms
总读取字节数: 3.00 MB
Stat 操作时间: 16.010µs
文件读取时间: 17.573ms
Stat 操作速率: 187382.89 文件/秒
读取吞吐量: 170.72 MB/秒
(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
已切换到目录: /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
当前工作目录: /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
path "./4" false
path "./4/5" false
path "./4/5/6" false
path "./4/5/1M_2_5.bin" true
path "./4/1M_2_4.bin" true
path "./1M_2_3.bin" true
共找到 3 个文件
./4/5/1M_2_5.bin
./4/1M_2_4.bin
./1M_2_3.bin

===== 性能统计 =====
文件数量: 3
cd 加载目录: 250.112ms
总读取字节数: 3.00 MB
Stat 操作时间: 14.346µs
文件读取时间: 22.745ms
Stat 操作速率: 209117.52 文件/秒
读取吞吐量: 131.90 MB/秒

总结：初次加载目录和文件内容，网络开销大

#测试加载缓存以后读取的性能
script/run /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
script/run /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3

(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
已切换到目录: /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
当前工作目录: /home/luxian/megadir/mount/third-party/test_202506162053/1/2/3
path "./4" false
path "./4/5" false
path "./4/5/6" false
path "./4/5/1M_1_5.bin" true
path "./4/1M_1_4.bin" true
path "./1M_1_3.bin" true
共找到 3 个文件
./4/5/1M_1_5.bin
./4/1M_1_4.bin
./1M_1_3.bin

===== 性能统计 =====
文件数量: 3
cd 加载目录: 36.546ms
总读取字节数: 3.00 MB
Stat 操作时间: 10.790µs
文件读取时间: 11.981ms
Stat 操作速率: 278035.22 文件/秒
读取吞吐量: 250.40 MB/秒

(base) root@yyjeqhc:~/git/mega/scorpio/script# ./run /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
已切换到目录: /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
当前工作目录: /home/luxian/megadir/mount/third-party/test_202506162053/2/2/3
path "./4" false
path "./4/5" false
path "./4/5/6" false
path "./4/5/1M_2_5.bin" true
path "./4/1M_2_4.bin" true
path "./1M_2_3.bin" true
共找到 3 个文件
./4/5/1M_2_5.bin
./4/1M_2_4.bin
./1M_2_3.bin

===== 性能统计 =====
文件数量: 3
cd 加载目录: 39.063ms
总读取字节数: 3.00 MB
Stat 操作时间: 15.379µs
文件读取时间: 20.243ms
Stat 操作速率: 195071.20 文件/秒
读取吞吐量: 148.20 MB/秒

总结：cd切换文件夹路径，如果文件夹内容没有变化，也至少有一次网络开销，但实际上，目录变化不会那么频繁，所以不用每次都要进行同步
```

