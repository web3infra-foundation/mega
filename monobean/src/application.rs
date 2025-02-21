use crate::config::WEBSITE;
use crate::CONTEXT;

use crate::core::mega_core::MegaCommands;
use crate::core::mega_core::MegaCommands::MegaStart;
use crate::window::MonobeanWindow;
use adw::gio::Settings;
use adw::glib::LogLevels;
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::unbounded;
use async_channel::{Receiver, Sender};
use gtk::glib::Priority;
use gtk::glib::{clone, WeakRef};
use gtk::{gio, glib};
use std::cell::{OnceCell, RefCell};
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

glib::wrapper! {
    pub struct MonobeanApplication(ObjectSubclass<imp::MonobeanApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

#[derive(Debug, Clone)]
pub enum Action {
    // Mega Core Related Actions
    MegaCore(MegaCommands),

    // Mega Frontend Related Actions
    AddToast(String),
    UpdateGitConfig(String, String),
    ShowHelloPage,
    ShowMainPage,
}

mod imp {
    use super::*;
    use crate::core::delegate::MegaDelegate;

    use crate::window::MonobeanWindow;

    pub struct MonobeanApplication {
        pub mega_delegate: &'static MegaDelegate,
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
            let mega_delegate = MegaDelegate::new(sender.clone());
            let settings = OnceCell::new();

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

            if let Some(weak_window) = self.window.get() {
                weak_window.upgrade().unwrap().present();
                return;
            }

            let window = app.create_window();
            self.window.set(window.downgrade()).unwrap();

            app.setup_log();

            // Setup action channel
            let receiver = self.receiver.borrow_mut().take().unwrap();
            CONTEXT.spawn_local_with_priority(
                Priority::HIGH,
                clone!(
                    #[strong]
                    app,
                    async move {
                        while let Ok(action) = receiver.recv().await {
                            app.process_action(action);
                        }
                    }
                ),
            );

            app.start_mega();

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

        self.add_window(&window);
        window.present();
        window
    }

    fn setup_gactions(&self) {
        let quit_action = gio::SimpleAction::new("quit", None);
        let about_action = gio::SimpleAction::new("about", None);

        quit_action.connect_activate(clone!(
            #[weak(rename_to = app)]
            self,
            move |_, _| {
                app.send_command(MegaCommands::MegaShutdown);
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

        self.add_action(&quit_action);
        self.add_action(&about_action);
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    pub fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
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
        let filter = tracing_subscriber::EnvFilter::new("warn,monobean=debug");
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(filter)
            .init();

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

                println!("{}: {}", level, fields);
            },
        );
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let dialog = gtk::AboutDialog::builder()
            .transient_for(&window)
            .modal(true)
            .program_name(crate::APP_NAME)
            .logo_icon_name("logo")
            .version(crate::config::VERSION)
            .authors(vec!["Neon"])
            .license_type(gtk::License::MitX11)
            .website(WEBSITE)
            .website_label("Github")
            .build();

        dialog.present();
    }

    pub fn send_command(&self, cmd: MegaCommands) {
        self.imp().mega_delegate.send_command(cmd);
    }

    pub fn start_mega(&self) {
        // The first Action of the application, so it can never block the gui thread.
        let http_addr = self
            .settings()
            .string("http-address")
            .to_value()
            .get::<String>()
            .unwrap();
        let http_port = self
            .settings()
            .uint("http-port")
            .to_value()
            .get::<u32>()
            .unwrap();
        let ssh_addr = self
            .settings()
            .string("ssh-address")
            .to_value()
            .get::<String>()
            .unwrap();
        let ssh_port = self
            .settings()
            .uint("ssh-port")
            .to_value()
            .get::<u32>()
            .unwrap();

        let http_addr = IpAddr::V4(http_addr.parse().unwrap());
        let ssh_addr = IpAddr::V4(ssh_addr.parse().unwrap());
        self.send_command(MegaStart(
            Option::from(SocketAddr::new(http_addr, http_port as u16)),
            Option::from(SocketAddr::new(ssh_addr, ssh_port as u16)),
        ));
    }

    fn process_action(&self, action: Action) {
        if self.active_window().is_none() {
            return;
        }

        match action {
            Action::MegaCore(cmd) => self.send_command(cmd),
            Action::AddToast(msg) => {
                self.window().unwrap().add_toast(msg);
            }
            Action::UpdateGitConfig(name, email) => {
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
                self.sender().send_blocking(toast).unwrap();
            }

            Action::ShowHelloPage => {
                let window = self.imp().window.get().unwrap().upgrade().unwrap();

                let stack = window.imp().base_stack.clone();
                stack.set_visible_child_name("hello_page");

                let config = self.git_config();
                let name = config.string("user.name").map(|name| name.to_string());
                let email = config.string("user.email").map(|email| email.to_string());

                window.show_hello_page(name, email);
            }

            Action::ShowMainPage => {
                let window = self.imp().window.get().unwrap().upgrade().unwrap();
                window.show_main_page();
            }
        }
    }
}
