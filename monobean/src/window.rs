use async_channel::Sender;
use gtk::CssProvider;
use gtk::{style_context_add_provider_for_display, PopoverMenu};

use crate::application::{Action, MonobeanApplication};
use crate::components::theme_selector::ThemeSelector;
use crate::components::{mega_tab::MegaTab, not_implemented::NotImplemented, repo_tab::RepoTab};
use crate::config::PREFIX;
use adw::glib::Priority;
use adw::prelude::{Cast, ObjectExt, SettingsExtManual, ToValue};
use adw::subclass::prelude::*;
use adw::{gio, ColorScheme, StyleManager, Toast};
use gtk::gio::Settings;
use gtk::glib;
use gtk::prelude::GtkWindowExt;
use gtk::CompositeTemplate;
use std::cell::OnceCell;

glib::wrapper! {
    pub struct MonobeanWindow(ObjectSubclass<imp::MonobeanWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

mod imp {
    use super::*;
    use crate::components::hello_page::HelloPage;
    use adw::glib::{ParamSpec, ParamSpecObject, Value};
    use std::cell::RefCell;
    use std::sync::LazyLock;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/window.ui")]
    pub struct MonobeanWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub base_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub primary_menu_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub hello_page: TemplateChild<HelloPage>,
        #[template_child]
        pub mega_tab: TemplateChild<MegaTab>,
        #[template_child]
        pub repo_tab: TemplateChild<RepoTab>,

        #[template_child]
        pub not_implemented: TemplateChild<NotImplemented>,

        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,

        toast: RefCell<Option<Toast>>,
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
            self.toast.replace(Some(Toast::new("")));

            obj.bind_settings();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecObject::builder::<Toast>("toast").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "toast" => {
                    let toast = value.get().unwrap();
                    self.toast.replace(toast);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "toast" => self.toast.borrow().to_value(),
                _ => unimplemented!(),
            }
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
        let window: MonobeanWindow = glib::Object::new();
        window.set_application(Some(application));
        window.imp().sender.set(sender).unwrap();

        window.setup_widget();
        // window.setup_action();
        window.setup_page();
        window
    }

    pub fn monobean_app(&self) -> MonobeanApplication {
        self.application().unwrap().downcast().unwrap()
    }

    fn sender(&self) -> Sender<Action> {
        self.imp().sender.get().unwrap().clone()
    }

    pub fn settings(&self) -> &Settings {
        self.imp()
            .settings
            .get_or_init(|| Settings::new(crate::APP_ID))
    }

    fn setup_page(&self) {
        let imp = self.imp();
        // let setting = self.settings();

        imp.hello_page.setup_hello_page(self.sender());

        // We are developing, so always show hello_page for debug
        let stack = imp.base_stack.clone();
        stack.set_visible_child_name("hello_page");

        let action = Action::ShowHelloPage;
        self.sender().send_blocking(action).unwrap();
    }

    pub fn show_main_page(&self) {
        let stack = self.imp().base_stack.clone();
        stack.set_visible_child_name("main_page");
    }

    pub fn show_hello_page(&self, name: Option<String>, email: Option<String>, pgp_generated: bool) {
        let stack = self.imp().base_stack.clone();
        let page = self.imp().hello_page.clone();
        page.fill_entries(name, email, pgp_generated);
        stack.set_visible_child_name("hello_page");
    }

    fn setup_widget(&self) {
        let imp = self.imp();
        let prim_btn = imp.primary_menu_button.get();
        let popover = prim_btn.popover().unwrap();
        let popover = popover.downcast::<PopoverMenu>().unwrap();
        let theme = ThemeSelector::new();
        popover.add_child(&theme, "theme");
    }

    pub fn add_toast(&self, message: String) {
        let pre = self.property::<Toast>("toast");

        let toast = Toast::builder()
            .title(glib::markup_escape_text(&message))
            .priority(adw::ToastPriority::High)
            .build();
        self.set_property("toast", &toast);
        self.imp().toast_overlay.add_toast(toast);

        // seems that dismiss will clear something used by animation
        // cause adw_animation_skip emit 'done' segfault on closure(https://github.com/gmg137/netease-cloud-music-gtk/issues/202)
        // delay to wait for animation skipped/done
        crate::CONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
            glib::timeout_future(std::time::Duration::from_millis(500)).await;
            // removed from overlay toast queue by signal
            pre.dismiss();
        });
    }

    fn bind_settings(&self) {
        let style = StyleManager::default();
        self.settings()
            .bind("style-variant", &style, "color-scheme")
            .mapping(|themes, _| {
                let themes = themes
                    .get::<String>()
                    .expect("The variant needs to be of type `String`.");
                let scheme = match themes.as_str() {
                    "system" => ColorScheme::Default,
                    "light" => ColorScheme::ForceLight,
                    "dark" => ColorScheme::ForceDark,
                    _ => ColorScheme::Default,
                };
                Some(scheme.to_value())
            })
            .build();
    }
}

fn load_css() {
    const CSS_FILES: [&str; 2] = ["tag.css", "common.css"];

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
