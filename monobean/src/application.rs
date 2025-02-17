use crate::config::WEBSITE;
use crate::CONTEXT;

use crate::core::mega_core::MegaCommands;
use crate::core::mega_core::MegaCommands::MegaStart;
use crate::window::MonobeanWindow;
use adw::gio::Settings;
use adw::glib::{LogLevel, LogLevels};
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::unbounded;
use async_channel::{Receiver, Sender};
use gtk::glib::Priority;
use gtk::glib::{clone, WeakRef};
use gtk::{gio, glib};
use std::cell::{OnceCell, RefCell};
use std::net::{IpAddr, SocketAddr};
use adw::glib::translate::IntoGlib;
use tracing::{event, Level};
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
    // Mega Frontend Related Actions
    AddToast(String),

    // Mega Core Related Actions
    MegaCore(MegaCommands),
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
            obj.bind_settings();
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

    fn bind_settings(&self) {
        // self.settings().bind("title", self, "window-title")
        //     .flags(glib::BindingFlags::SYNC_CREATE)
        //     .build();
    }

    fn setup_log(&self) {
        // TODO: Use gtk settings for log level.
        let filter = tracing_subscriber::EnvFilter::new("warn,monobean=debug");
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(filter)
            .init();
        
        glib::log_set_handler(
            Some(crate::APP_ID),
            LogLevels::all(),
            false,
            false, 
            |_, glib_level,msg| {
                let glib_level = LogLevels::from_bits(glib_level.into_glib()).unwrap();
                match glib_level {
                    LogLevels::LEVEL_CRITICAL | LogLevels::LEVEL_ERROR => tracing::error!(target: "monobean", "{}", msg),
                    LogLevels::LEVEL_WARNING => tracing::warn!(target: "monobean", "{}", msg),
                    LogLevels::LEVEL_MESSAGE => tracing::info!(target: "monobean", "{}", msg),
                    LogLevels::LEVEL_INFO => tracing::debug!(target: "monobean", "{}", msg),
                    _ => tracing::trace!(target: "monobean", "{}", msg),
                };
            }
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
            Action::AddToast(msg) => {
                self.window().unwrap().add_toast(msg);
            }

            Action::MegaCore(cmd) => self.send_command(cmd),
        }
    }
}
