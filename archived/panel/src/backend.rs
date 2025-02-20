use common::config::Config as MegaConfig;
use common::model::{CommonOptions, ZtmOptions};
use gateway::https_server::{http_server, HttpOptions};
use jupiter::context::Context as MegaContext;

pub(crate) async fn init(config: &MegaConfig) {
    let ctx = MegaContext::new(config.clone());

    // FIXME: Add options field into config file
    let common = CommonOptions {
        host: String::from("127.0.0.1"),
    };
    let ztm = ZtmOptions {
        ztm_agent_port: 7777,
        bootstrap_node: None,
        cache: false,
    };
    let opt = HttpOptions {
        common,
        ztm,
        http_port: 8000,
    };

    tokio::spawn(async move { http_server(ctx.await, opt).await });
}
