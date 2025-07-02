use crate::config::{
    config_update, load_mega_resource, monobean_base, monobean_cache, MEGA_CONFIG_PATH, WEBSITE,
};
use crate::core::delegate::MegaDelegate;
use crate::core::CoreConfigChanged;
use crate::{get_setting, CONTEXT};

use crate::components::preference::MonobeanPreferences;
use crate::core::mega_core::MegaCommands;
use crate::core::mega_core::MegaCommands::MegaStart;
use crate::window::MonobeanWindow;
use adw::gio::Settings;
use adw::glib::LogLevels;
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::unbounded;
use async_channel::{Receiver, Sender};
use common::config::Config;
use common::model::P2pOptions;
use gtk::glib::Priority;
use gtk::glib::{clone, WeakRef};
use gtk::{gio, glib};
use std::cell::{OnceCell, RefCell};
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::oneshot;
use tracing_subscriber::fmt::writer::MakeWriterExt;

glib::wrapper! {
    pub struct MonobeanApplication(ObjectSubclass<imp::MonobeanApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

#[derive(Debug)]
pub enum Action {
    // Mega Core Related Actions
    MegaCore(MegaCommands),

    // Mega Frontend Related Actions
    AddToast(String),
    UpdateGitConfig(String, String),
    ShowHelloPage,
    ShowMainPage,
    MountRepo,
    OpenEditorOn { hash: String, name: String },
}

mod imp {

    use super::*;

    use crate::core::delegate::MegaDelegate;

    use crate::window::MonobeanWindow;

    pub struct MonobeanApplication {
        pub mega_delegate: OnceCell<MegaDelegate>,
        pub window: OnceCell<WeakRef<MonobeanWindow>>,
        pub sender: Sender<Action>,
        pub receiver: RefCell<Option<Receiver<Action>>>,
        pub settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MonobeanApplication {
        const NAME: &'static str = "MonobeanApplication";
        type Type = super::MonobeanApplication;
        type ParentType = adw::Application;

        fn new() -> Self {
            let (sender, r) = unbounded();
            let receiver = RefCell::new(Some(r));
            let window = OnceCell::new();
            let settings = OnceCell::new();
            let mega_delegate = OnceCell::new();

            Self {
                mega_delegate,
                window,
                sender,
                receiver,
                settings,
            }
        }
    }

    impl ObjectImpl for MonobeanApplication {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.setup_settings();
            obj.setup_mega();
            // obj.bind_settings();
            obj.setup_gactions();
        }
    }

    impl ApplicationImpl for MonobeanApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let obj = self.obj();

            let app = obj.downcast_ref::<super::MonobeanApplication>().unwrap();
            app.setup_log();

            if let Some(weak_window) = self.window.get() {
                weak_window.upgrade().unwrap().present();
                tracing::error!("Window already exists.");
                return;
            }

            let window = app.create_window();
            self.window.set(window.downgrade()).unwrap();

            // Setup action channel
            let receiver = self.receiver.borrow_mut().take().unwrap();
            CONTEXT.spawn_local_with_priority(
                Priority::HIGH,
                clone!(
                    #[strong]
                    app,
                    async move {
                        app.start_mega().await;
                        while let Ok(action) = receiver.recv().await {
                            tracing::debug!("Processing Glib Action {:?}", action);
                            app.process_action(action);
                        }
                    }
                ),
            );

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for MonobeanApplication {}
    impl AdwApplicationImpl for MonobeanApplication {}
}

impl MonobeanApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    pub fn sender(&self) -> Sender<Action> {
        self.imp().sender.clone()
    }

    pub fn window(&self) -> Option<MonobeanWindow> {
        self.imp()
            .window
            .get()
            .map(|w| w.upgrade().expect("Window not setup yet."))
    }

    fn create_window(&self) -> MonobeanWindow {
        let window = MonobeanWindow::new(&self.clone(), self.sender());

        window.set_decorated(true);
        window.set_icon_name(Some("mono-white-logo"));
        self.add_window(&window);
        window.present();
        window
    }

    fn setup_gactions(&self) {
        let quit_action = gio::SimpleAction::new("quit", None);
        let about_action = gio::SimpleAction::new("about", None);
        let preference_action = gio::SimpleAction::new("preference", None);

        quit_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.blocking_send_command(MegaCommands::MegaShutdown);
                app.quit();
            }
        ));

        about_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.show_about();
            }
        ));

        preference_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.show_preference();
            }
        ));

        self.add_action(&quit_action);
        self.add_action(&about_action);
        self.add_action(&preference_action);
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn setup_mega(&self) {
        // TODO: use `Config::load_sources` to load glib shcema
        let bytes = load_mega_resource(MEGA_CONFIG_PATH);
        let content = String::from_utf8(bytes).expect("Mega core setting must be in utf-8");
        let config =
            Config::load_str(content.as_str()).expect("Failed to parse mega core settings");

        let delegate = MegaDelegate::new(self.sender(), config);
        self.imp()
            .mega_delegate
            .set(delegate)
            .expect("Could not set `MegaDelegate`.");

        self.apply_user_config();
    }

    pub fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    pub fn delegate(&self) -> &MegaDelegate {
        self.imp()
            .mega_delegate
            .get()
            .expect("Could not get `MegaDelegate`.")
    }

    pub fn git_config(&self) -> gix_config::File<'static> {
        let home_dir = home::home_dir().expect("Cannot find home directory");
        let target = home_dir.join(".gitconfig");
        if !target.exists() {
            std::fs::File::create_new(&target).unwrap();
        }

        gix_config::File::from_path_no_includes(target, gix_config::Source::User).unwrap()
    }

    fn setup_log(&self) {
        // TODO: Use gtk settings for log level.
        // FIXME: currently not working for glib logs.
        let file_appender =
            tracing_appender::rolling::hourly(monobean_cache(), "monobean-logs.txt");
        let filter = tracing_subscriber::EnvFilter::new("info,monobean=debug");

        if cfg!(debug_assertions) {
            tracing_subscriber::fmt()
                .with_writer(file_appender.and(std::io::stdout))
                .with_env_filter(filter)
                .with_target(false)
                .with_line_number(true)
                .with_file(true)
                .without_time()
                .compact()
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_writer(file_appender)
                .with_env_filter(filter)
                .with_target(false)
                .without_time()
                .compact()
                .init();
        }

        glib::log_set_handler(
            None,
            LogLevels::all(),
            true,
            true,
            |_domain, log_level, fields| {
                let level = match log_level {
                    glib::LogLevel::Error => tracing::Level::ERROR,
                    glib::LogLevel::Critical => tracing::Level::ERROR,
                    glib::LogLevel::Warning => tracing::Level::WARN,
                    glib::LogLevel::Message => tracing::Level::INFO,
                    glib::LogLevel::Info => tracing::Level::INFO,
                    glib::LogLevel::Debug => tracing::Level::DEBUG,
                };

                println!("{level}: {fields}");
            },
        );
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .program_name(crate::APP_NAME)
            .logo_icon_name("mono-white-logo")
            .version(crate::config::VERSION)
            .authors(vec!["Neon"])
            .license_type(gtk::License::MitX11)
            .website(WEBSITE)
            .website_label("Github")
            .build();

        dialog.present();
    }

    fn show_preference(&self) {
        let window = self.window();
        let dialog = MonobeanPreferences::new();
        dialog.set_transient_for(window.as_ref());
        dialog.set_modal(true);

        GtkWindowExt::present(&dialog);
    }

    pub async fn send_command(&self, cmd: MegaCommands) {
        self.delegate().send_command(cmd).await;
    }

    pub fn blocking_send_command(&self, cmd: MegaCommands) {
        self.delegate().blocking_send_command(cmd);
    }

    pub async fn start_mega(&self) {
        let settings = self.settings();

        let bootstrap_node = get_setting!(settings, "bootstrap-node", String);
        let http_addr = get_setting!(settings, "http-address", String);
        let http_port = get_setting!(settings, "http-port", u32);
        let ssh_addr = get_setting!(settings, "ssh-address", String);
        let ssh_port = get_setting!(self.settings(), "ssh-port", u32);

        let http_addr = IpAddr::V4(http_addr.parse().unwrap());
        let ssh_addr = IpAddr::V4(ssh_addr.parse().unwrap());
        let p2p_opt = if bootstrap_node.is_empty() {
            P2pOptions::default()
        } else {
            P2pOptions {
                bootstrap_node: Some(bootstrap_node),
            }
        };

        self.send_command(MegaStart(
            Option::from(SocketAddr::new(http_addr, http_port as u16)),
            Option::from(SocketAddr::new(ssh_addr, ssh_port as u16)),
            p2p_opt,
        ))
        .await;
    }

    /// Send a command to mega core
    ///
    /// # Warning:
    /// May stuck main event loop.
    pub(crate) fn core_status(&self) -> oneshot::Receiver<(bool, bool)> {
        let (tx, rx) = oneshot::channel();
        let cmd = MegaCommands::CoreStatus(tx);
        let act = Action::MegaCore(cmd);
        self.sender().send_blocking(act).unwrap();
        rx
    }

    fn apply_user_config(&self) {
        let mut updates = Vec::new();

        // Always ignore the base_dir in config.toml
        // https://github.com/web3infra-foundation/mega/issues/957
        updates.push(CoreConfigChanged::BaseDir(monobean_base()));
        updates.push(CoreConfigChanged::LogPath(monobean_cache().join("logs")));
        updates.push(CoreConfigChanged::DbPath(
            monobean_base().join("monobean.db"),
        ));

        // However, this behavior can still be overridden from GUI preferences.
        updates.extend(config_update(self.settings()));
        self.blocking_send_command(MegaCommands::ApplyUserConfig(updates));
    }

    fn process_action(&self, action: Action) {
        if self.active_window().is_none() {
            return;
        }

        let window = self.imp().window.get().unwrap().upgrade().unwrap();
        match action {
            Action::MegaCore(cmd) => {
                let delegate = self.delegate().clone();
                CONTEXT.spawn(async move {
                    tracing::debug!("Sending {:?}", cmd);
                    delegate.send_command(cmd).await;
                    tracing::debug!("Done");
                });
            }
            Action::AddToast(msg) => {
                window.add_toast(msg);
            }
            Action::UpdateGitConfig(name, email) => {
                let sender = self.sender();
                let mut config = self.git_config();
                config.set_raw_value(&"user.name", name.as_bytes()).unwrap();
                config
                    .set_raw_value(&"user.email", email.as_bytes())
                    .unwrap();

                // gix_config does not write back to file automatically
                // so we need to write it back manually.
                let loc = config.meta().path.clone().unwrap();
                let mut fd = std::fs::File::create(loc).unwrap();
                config.write_to(&mut fd).unwrap();
                tracing::debug!("Git config: {:?}", config.meta());

                let toast = Action::AddToast("Git config updated!".to_string());
                CONTEXT.spawn(async move {
                    sender.send(toast).await.unwrap();
                });
            }
            Action::ShowHelloPage => {
                let config = self.git_config();

                let name = config.string("user.name").map(|name| name.to_string());
                let email = config.string("user.email").map(|email| email.to_string());

                let rx = self.core_status();
                CONTEXT.spawn_local(async move {
                    let (_, gpg_generated) = rx.await.unwrap();
                    window.show_hello_page(name, email, gpg_generated);
                });
            }
            Action::ShowMainPage => {
                window.show_main_page();
            }
            Action::MountRepo => todo!(),
            Action::OpenEditorOn { hash, name } => {
                CONTEXT.spawn_local(async move {
                    let window = window.imp();
                    let code_page = window.code_page.get();
                    code_page.show_editor_on(hash, name);
                });
            }
        }
    }
}
