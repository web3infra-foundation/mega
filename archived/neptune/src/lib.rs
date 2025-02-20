/// a test demo for pipy
use libc::{c_char, c_int};
use std::ffi::CString;
use std::thread;

#[link(name = "pipy", kind = "dylib")]
extern "C" {
    pub fn pipy_main(argc: c_int, argv: *const *const c_char) -> c_int;

    pub fn pipy_exit(force: c_int);
}

/// start ztm agent, like run pipy repo://ztm/agent --database=database --listen=0.0.0.0:listen_port
/// ! only support to start one agent or one hub at one process
pub fn start_agent(database: &str, listen_port: u16) {
    let _database = database.to_string();
    tracing::info!("start pipy with port: {}", listen_port);
    let args = [
        CString::new("ztm-pipy").unwrap(),
        CString::new("repo://ztm/agent").unwrap(),
        // CString::new("--reuse-port").unwrap(),
        CString::new("--args").unwrap(),
        CString::new("--data").unwrap(),
        CString::new(database).unwrap(),
        CString::new("--listen").unwrap(),
        CString::new(format!("127.0.0.1:{}", listen_port)).unwrap(),
    ];
    let c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    unsafe {
        pipy_main(c_args.len() as c_int, c_args.as_ptr());
    }
    thread::sleep(std::time::Duration::from_secs(1)); // wait for pipy to start
}

/// start ztm hub, like run pipy repo://ztm/hub --listen=0.0.0.0:listen_port --name=name --ca=ca
/// ! only support to start one agent or one hub at one process
pub fn start_hub(listen_port: u16, name: Vec<String>, _ca: &str) {
    let _ = name; // TODO: ignore name
    tracing::info!("start pipy with port: {}", listen_port);
    let args = [
        CString::new("ztm-pipy").unwrap(),
        CString::new("repo://ztm/hub").unwrap(),
        CString::new("--args").unwrap(),
        CString::new("--listen").unwrap(),
        CString::new(format!("127.0.0.1:{}", listen_port)).unwrap(),
        // CString::new(format!("--ca={}", ca)).unwrap(),
    ];
    let c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    unsafe {
        pipy_main(c_args.len() as c_int, c_args.as_ptr());
    }
}

/// exit ztm agent or hub
pub fn exit_ztm() {
    unsafe {
        pipy_exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_agent() {
        let port = 7776;
        start_agent("test.db", port);
        thread::sleep(std::time::Duration::from_secs(1));

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/version", port))
            .await
            .unwrap();
        tracing::debug!("resp: {:?}", resp);
        assert!(resp.status().is_success());
        tracing::info!("ztm agent start success");

        exit_ztm();
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/version", port))
            .await
            .unwrap();
        tracing::debug!("resp: {:?}", resp);
        assert!(resp.status().as_u16() == 502); // 502
        tracing::info!("ztm agent exit success");
    }

    #[tokio::test]
    async fn test_start_hub() {
        start_hub(8888, vec![], "localhost:9999");
        thread::sleep(std::time::Duration::from_secs(3));
    }

    #[tokio::test]
    #[should_panic]
    /// didn't support multiple agent
    async fn test_start_multiple_agent() {
        let port1 = 7777;
        let port2 = 7778;
        start_agent("test1.db", port1);

        let resp = reqwest::get(format!("http://0.0.0.0:{}/api/version", port1))
            .await
            .unwrap();
        assert!(resp.status().is_success());

        start_agent("test2.db", port2);
        let resp = reqwest::get(format!("http://0.0.0.0:{}/api/version", port2))
            .await
            .unwrap();
        assert!(resp.status().is_success());

        exit_ztm();

        let resp = reqwest::get(format!("http://http://0.0.0.0:{}/api/version", port1))
            .await
            .unwrap();
        assert!(resp.status().as_u16() == 502); // 502
    }
}
