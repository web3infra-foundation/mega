mod assets;
mod window;

use crate::frontend::assets::Assets;
use common::config::Config as MegaConfig;
use gpui::App;

pub(crate) async fn init(_config: &MegaConfig) {
    let app = App::new().with_assets(Assets);

    app.run(move |_cx| {
        
    });
}
