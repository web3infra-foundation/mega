use std::sync::Arc;

use hyper::body::Bytes;
use hyper::{body::HttpBody as _, client::HttpConnector, Client};
use hyper::{Body, Error, Method, Request, StatusCode, Uri};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

pub struct RemoteServer {
    base_url: String,
    rt: Arc<Runtime>,
    http_client: Client<HttpConnector>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InodeContent {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub permissions: u16,
}

impl RemoteServer {
    pub fn new(addr: &str, rt: Arc<Runtime>) -> Self {
        Self {
            base_url: addr.to_string(),
            rt,
            http_client: Client::new(),
        }
    }

    pub fn list(&self, path: String) -> Option<Vec<InodeContent>> {
        let url = self.base_url.clone() + "?repo_path=" + &path;
        match self.req(url, true) {
            Ok((status, body)) => {
                if status != 200 || body.is_none() {
                    return None;
                }
                let mut content = String::new();
                for item in body.unwrap() {
                    let segment = String::from_utf8(item.to_vec()).unwrap();
                    content += &segment;
                }
                Some(serde_json::from_str(&content).unwrap())
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    pub fn create(&self, path: &str, kind: &str) -> Option<()> {
        let url =
            self.base_url.clone() + "?repo_path=" + path + "&kind=" + kind + "&operation=create";
        match self.req(url, false) {
            Ok((status, _)) => {
                if status != 200 {
                    None
                } else {
                    Some(())
                }
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    pub fn delete(&self, path: &str, kind: &str) -> Option<()> {
        let url =
            self.base_url.clone() + "?repo_path=" + path + "&kind=" + kind + "&operation=delete";
        match self.req(url, false) {
            Ok((status, _)) => {
                if status != 200 {
                    None
                } else {
                    Some(())
                }
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    pub fn alter(&self, path: &str, kind: &str) -> Option<()> {
        let url = self.base_url.clone().clone()
            + "?repo_path="
            + path
            + "&kind="
            + kind
            + "&operation=alter";
        match self.req(url, false) {
            Ok((status, _)) => {
                if status != 200 {
                    None
                } else {
                    Some(())
                }
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    pub async fn commit_change(&self, content: String) {
        let uri: Uri = (self.base_url.clone() + "?ops=commit").parse().unwrap();
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(content))
            .unwrap();
        match self.http_client.request(req).await {
            Ok(resp) => {
                if resp.status() == !200 {
                    println!("commit error");
                }
            }
            Err(e) => {
                println!("{}", e);
            }
        }
    }

    pub fn download(&self, id: String) -> Option<Vec<Bytes>> {
        let url = self.base_url.clone() + "?object_id=" + &id;
        match self.req(url, true) {
            Ok((status, body)) => {
                if status != 200 {
                    None
                } else {
                    body
                }
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    pub fn mv(&self, path: &str, new_path: &str) -> Option<()> {
        let url = self.base_url.clone() + "?repo_path=" + path + "&new_path=" + new_path;
        match self.req(url, false) {
            Ok((status, _)) => {
                if status != 200 {
                    None
                } else {
                    Some(())
                }
            }
            Err(e) => {
                println!("{}", e);
                None
            }
        }
    }

    fn req(
        &self,
        url: String,
        body_option: bool,
    ) -> Result<(StatusCode, Option<Vec<Bytes>>), Error> {
        self.rt.block_on(async {
            let mut resp = self.http_client.get(url.parse().unwrap()).await?;
            let status = resp.status();
            if !body_option {
                return Ok((status, None));
            }

            let mut res: Vec<Bytes> = Vec::new();
            while let Some(chunk) = resp.body_mut().data().await {
                match chunk {
                    Ok(bytes) => res.push(bytes),
                    Err(e) => return Err(e),
                }
            }
            Ok((status, Some(res)))
        })
    }
}
