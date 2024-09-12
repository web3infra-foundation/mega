
use std::{collections::VecDeque, sync::Arc, time::Duration};
use mercury::internal::object::tree::{Tree, TreeItemMode};
use reqwest::Client;
use tokio::sync::Mutex;
use tokio::time;

use crate::util::GPath;
const BASE_URL : &str = "http://localhost:8000/api/v1/file/tree?path=/";
#[allow(unused)]
#[allow(clippy::blocks_in_conditions)]
async fn worker_thread(
    shared_queue: Arc<Mutex<VecDeque<GPath>>>
) {
    let client = Client::new();
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        // 尝试从队列中获取路径，超时设置为 5 秒
        let path = match time::timeout(Duration::from_secs(5), async {
            let mut queue = shared_queue.lock().await;
            queue.pop_front()
        }).await {
            Ok(Some(path)) => path,
            Ok(None) => {
                // 如果队列为空，继续等待
                interval.tick().await;
                continue;
            },
            Err(_) => {
                // 处理超时错误（例如：可以选择打印日志或退出）
                println!("Timeout occurred while waiting for path");
                continue;
            },
        };

        // 处理路径
        let url = format!("{}{}", BASE_URL, path);
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            match Tree::try_from(&bytes[..]) {
                                Ok(tree) => {
                                    trace!("path:{},new tree:{}",path,tree );
                                    for item in tree.tree_items {
                                        if item.mode == TreeItemMode::Tree {
                                            let mut subpath = path.clone();
                                            subpath.push(item.name);
                                            {
                                                let mut queue = shared_queue.lock().await;
                                                queue.push_back(subpath);
                                            }
                                            
                                        } else {
                                            // 处理文件
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("Failed to parse tree: {:?}", e);
                                },
                            }
                        },
                        Err(e) => {
                            println!("Failed to get response bytes: {:?}", e);
                        },
                    }
                } else {
                    println!("Failed to fetch tree: {}", response.status());
                }
            },
            Err(e) => {
                println!("Failed to send request: {:?}", e);
            },
        }

        // 等待下一次轮询
        interval.tick().await;
    }
}

#[cfg(test)]
mod tests2 {
    use reqwest::Client;
    use tokio::{sync::Mutex, time};
    use std::{collections::VecDeque, error::Error, sync::Arc};
    use mercury::internal::object::tree::Tree;
    use crate::{manager::fetch::worker_thread, util::GPath};
    #[tokio::test]
    async fn test_fetch_octet_stream() -> Result<(), Box<dyn Error>> {
        // 创建 HTTP 客户端
        let client = Client::new();
        
        // 使用环境变量中的 URL 或者本地测试 URL
        let url = "http://localhost:8000/api/v1/file/tree?path=/third-part/";
        
        // 发送 GET 请求
        let response = client.get(url).send().await?;
        
        // 确保响应状态是成功的
        if response.status().is_success() {
            // 获取响应体的二进制数据
            let content = response.bytes().await?;
            
            // 将内容存入 Vec<u8>
            let data: Vec<u8> = content.to_vec();
            let tree = Tree::try_from(&data[..]).unwrap();
            // 输出数据长度用于测试断言
            // println!("Received {} bytes of data", data.len());
            
            // // 这里可以添加更多的断言或验证逻辑
            // assert!(!data.is_empty(), "Data should not be empty");

                println!("{}",tree);
            // 可能还可以验证数据的具体内容
            // assert_eq!(data, expected_data); // 你需要定义 expected_data
            
        } else {
            eprintln!("Request failed with status: {}", response.status());
            return Err(format!("Request failed with status: {}", response.status()).into());
        }
        
        Ok(())
    }
    #[tokio::test]
    async fn test_fetch_tree( ){
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue_clone = queue.clone();
        let _handle = tokio::spawn(async move {
            // Initialize the queue with a path
            let mut queue = queue_clone.lock().await;
            let mut path = GPath::new();
            path.push(String::from("third-part"));
            queue.push_back(path);
        });
        let mut handles = vec![];
        for _ in 0..6 {
            let queue_clone = queue.clone();
            let handle = tokio::spawn(worker_thread(queue_clone));
            handles.push(handle);
        }
        // Wait for a while to let the workers process
        time::sleep(std::time::Duration::from_secs(5)).await;

                    // Clean up workers (depends on how you implement worker_thread termination)
        for handle in handles {
            let _ = handle.await;
        }
        
        // Check if the queue has been populated
        let queue = queue.lock().await;
        assert!(queue.len() > 0);

        
    }
}