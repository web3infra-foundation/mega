use crate::config::WEBSITE;
use crate::CONTEXT;
use crate::{mega::MegaCore, window::MonobeanWindow};

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::{Receiver, Sender};
use gtk::glib::{clone, WeakRef};
use gtk::{gio, glib};
use std::cell::{OnceCell, RefCell};
use std::path::PathBuf;

glib::wrapper! {
    pub struct MonobeanApplication(ObjectSubclass<imp::MonobeanApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

#[derive(Debug, Clone)]
pub enum Action {
    // Mega Frontend Related Actions
    AddToast(String),

    // Mega Backend Related Actions
    MegaShutdown,
    MegaRestart,
    FuseMount(PathBuf),
    FuseUnmount,
    SaveFileChange(PathBuf),
}

mod imp {

    use async_channel::unbounded;
    use gtk::glib::{clone, Priority};

    use super::*;

    pub struct MonobeanApplication {
        pub mega_core: OnceCell<MegaCore>,
        pub window: OnceCell<WeakRef<MonobeanWindow>>,
        pub sender: Sender<Action>,
        pub receiver: RefCell<Option<Receiver<Action>>>,
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
            let mega_core = OnceCell::new();


            Self {
                mega_core,
                window,
                sender,
                receiver,
            }
        }
    }

    impl ObjectImpl for MonobeanApplication {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

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
            let app = obj
                .downcast_ref::<super::MonobeanApplication>()
                .unwrap();

            if let Some(weak_window) = self.window.get() {
                weak_window.upgrade().unwrap().present();
                return;
            }

            let window = app.create_window();
            let _ = self.window.set(window.downgrade());
            let _ = self.mega_core.set(MegaCore::new());

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
        self.imp().window.get().map(|w| w.upgrade().expect("Window not setup yet."))
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

    fn process_action(&self, action: Action) {
        if self.active_window().is_none() {
            return;
        }

        match action {
            Action::AddToast(_) => todo!(),
            Action::MegaShutdown => todo!(),
            Action::MegaRestart => todo!(),
            Action::FuseMount(_path_buf) => todo!(),
            Action::FuseUnmount => todo!(),
            Action::SaveFileChange(_path_buf) => todo!(),
        }
    }
}
