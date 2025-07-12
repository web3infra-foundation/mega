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



优化fetch_code之前，1000个文件拉取测试

文件创建脚本：script/run_1000_files.sh

```sh
server running...ID:96cc1dd3-d9a8-453c-8976-4796e579a1d7|[init] REQRequest { unique: 2, uid: 0, gid: 0, pid: 0 }-  - Call:
ID:96cc1dd3-d9a8-453c-8976-4796e579a1d7 [init] - Success: ReplyInit { max_write: 131072 }
new tree:903752ee634b1708c4d046a40cc62e572bb87a32
new tree:68dfb43f330a0990660fbe5951be4cbbe4fda88e
new tree:29e9768f1ae1595cfc8229815a252adca1b75465
new tree:03d5ccb9b5b209ba7d69fb5b3cc440c477847451
new tree:15d966af87d8be73d17d0275e1e144ee1fd41e89
new tree:352fa9971717d83086b387df815e1290ee1722a4
new tree:49c68383b97e3ea515fd97c1badf4ddcc2d44851
new tree:98218ce618733c6b21ae092cd2eb9c490a6f51fa
new tree:4bf75ab6553e47a5a13beb6af0168382626e008f
new tree:4078d08857c555366433a97f0492139751192b67
new tree:370ce8cc1c2a1a70ed8605d16f3b2ebb2070c98d
new tree:ef3ab426ae85fc791cc7bc73f46c2847bcbeca90
new tree:79648eba285af8fe672abc2447591665d37ac49f
finish store....
finish code for third-party/t1...fetch 1000 files finished,use time: 68.8379431s

new tree:903752ee634b1708c4d046a40cc62e572bb87a32
new tree:68dfb43f330a0990660fbe5951be4cbbe4fda88e
new tree:29e9768f1ae1595cfc8229815a252adca1b75465
new tree:03d5ccb9b5b209ba7d69fb5b3cc440c477847451
new tree:15d966af87d8be73d17d0275e1e144ee1fd41e89
new tree:98218ce618733c6b21ae092cd2eb9c490a6f51fa
new tree:352fa9971717d83086b387df815e1290ee1722a4
new tree:4bf75ab6553e47a5a13beb6af0168382626e008f
new tree:49c68383b97e3ea515fd97c1badf4ddcc2d44851
new tree:370ce8cc1c2a1a70ed8605d16f3b2ebb2070c98d
new tree:4078d08857c555366433a97f0492139751192b67
new tree:ef3ab426ae85fc791cc7bc73f46c2847bcbeca90
new tree:79648eba285af8fe672abc2447591665d37ac49f
finish store....
finish code for third-party/t1...fetch 1000 files finished,use time: 65.558764133s

new tree:903752ee634b1708c4d046a40cc62e572bb87a32
new tree:68dfb43f330a0990660fbe5951be4cbbe4fda88e
new tree:29e9768f1ae1595cfc8229815a252adca1b75465
new tree:03d5ccb9b5b209ba7d69fb5b3cc440c477847451
new tree:15d966af87d8be73d17d0275e1e144ee1fd41e89
new tree:98218ce618733c6b21ae092cd2eb9c490a6f51fa
new tree:352fa9971717d83086b387df815e1290ee1722a4
new tree:4bf75ab6553e47a5a13beb6af0168382626e008f
new tree:49c68383b97e3ea515fd97c1badf4ddcc2d44851
new tree:4078d08857c555366433a97f0492139751192b67
new tree:370ce8cc1c2a1a70ed8605d16f3b2ebb2070c98d
new tree:79648eba285af8fe672abc2447591665d37ac49f
new tree:ef3ab426ae85fc791cc7bc73f46c2847bcbeca90
finish store....
finish code for third-party/t1...fetch 1000 files finished,use time: 66.745361716s
```

优化思路：

添加一个全局的文件下载管理器，维护一个下载队列，在遍历目录的过程中，将需要下载的内容加载到队列中，此队列有10个固定协程进行下载

优化结果：

```sh
Worker 4 shutting down
new tree:79648eba285af8fe672abc2447591665d37ac49f
Worker 0 shutting down
Worker 2 shutting down
Worker 3 shutting down
Worker 1 shutting down
finish store....
Finished downloading code for third-party/t1
fetch 1000 files finished,use time: 42.854706696s

Worker 4 shutting down
finish store....
Finished downloading code for third-party/t1
fetch 1000 files finished,use time: 41.484116077s

Worker 2 shutting down
Worker 0 shutting down
Worker 4 shutting down
new tree:79648eba285af8fe672abc2447591665d37ac49f
Worker 1 shutting down
Worker 3 shutting down
finish store....
Finished downloading code for third-party/t1
fetch 1000 files finished,use time: 44.768321595s


new tree:ef3ab426ae85fc791cc7bc73f46c2847bcbeca90
Worker 0 shutting down
Worker 1 shutting down
Worker 4 shutting down
Directory processing completed for third-party/t1
finish store....
Finished downloading code for third-party/t1
fetch 1000 files finished,use time: 45.409070254s

Worker 0 shutting down
Worker 3 shutting down
Worker 1 shutting down
Worker 4 shutting down
new tree:ef3ab426ae85fc791cc7bc73f46c2847bcbeca90
Worker 2 shutting down
Directory processing completed for third-party/t1
finish store....
Finished downloading code for third-party/t1
fetch 1000 files finished,use time: 52.19500406s
```

由于系统本身性能存在波动，综合来看，大概可以实现50%的下载提速
