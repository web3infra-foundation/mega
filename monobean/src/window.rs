use async_channel::Sender;
use gtk::style_context_add_provider_for_display;
use gtk::CssProvider;

use crate::components::{mega_tab::MegaTab, not_implemented::NotImplemented, repo_tab::RepoTab};
use crate::PREFIX;
use adw::gio;
use adw::subclass::prelude::*;
use gtk::gio::Settings;
use gtk::glib;
use gtk::CompositeTemplate;
use std::cell::OnceCell;

use crate::application::Action;

glib::wrapper! {
    pub struct MonobeanWindow(ObjectSubclass<imp::MonobeanWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/window.ui")]
    pub struct MonobeanWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub primary_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub mega_tab: TemplateChild<MegaTab>,
        #[template_child]
        pub repo_tab: TemplateChild<RepoTab>,

        #[template_child]
        pub not_implemented: TemplateChild<NotImplemented>,

        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MonobeanWindow {
        const NAME: &'static str = "MonobeanWindow";
        type Type = super::MonobeanWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            load_css();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MonobeanWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.setup_settings();
            obj.bind_settings();
        }
    }
    impl WidgetImpl for MonobeanWindow {}
    impl WindowImpl for MonobeanWindow {}
    impl ApplicationWindowImpl for MonobeanWindow {}
    impl AdwApplicationWindowImpl for MonobeanWindow {}
}

impl MonobeanWindow {
    pub fn new<P: glib::object::IsA<gtk::Application>>(
        application: &P,
        sender: Sender<Action>,
    ) -> Self {
        let window: MonobeanWindow = glib::Object::builder()
            .property("application", application)
            .build();

        window.imp().sender.set(sender).unwrap();
        // window.setup_widget();
        // window.setup_action();
        // window.init_page_data();
        window
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
}

fn load_css() {
    const CSS_FILES: [&str; 2] = ["tag.css", "card.css"];

    let _ = CSS_FILES
        .into_iter()
        .map(|f| {
            let provider = CssProvider::new();
            provider.load_from_resource(&format!("{}/css/{}", PREFIX, f));
            style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not connect to a display."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        })
        .collect::<Vec<_>>();
}
