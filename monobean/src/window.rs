use async_channel::Sender;
use gtk::CssProvider;
use gtk::{style_context_add_provider_for_display, PopoverMenu};

use crate::application::{Action, MonobeanApplication};
use crate::components::mega_tab::MegaTabWindow;
use crate::components::theme_selector::ThemeSelector;
use crate::config::PREFIX;
use crate::CONTEXT;
use adw::glib::{clone, Priority};
use adw::prelude::{
    ActionMapExt, Cast, ObjectExt, SettingsExt, SettingsExtManual, StaticVariantType, ToValue,
    ToVariant,
};
use adw::subclass::prelude::*;
use adw::{gio, ColorScheme, StyleManager, Toast};
use gtk::gio::Settings;
use gtk::gio::{SimpleAction, SimpleActionGroup};
use gtk::glib;
use gtk::prelude::{ButtonExt, GtkWindowExt, WidgetExt};
use gtk::CompositeTemplate;
use std::cell::OnceCell;

glib::wrapper! {
    pub struct MonobeanWindow(ObjectSubclass<imp::MonobeanWindow>)
        @extends gtk::Widget, gtk::Window, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

mod imp {
    use super::*;
    use crate::components::code_page::CodePage;
    use crate::components::hello_page::HelloPage;
    // use crate::components::not_implemented::NotImplemented;
    use crate::components::not_implemented::NotImplemented;
    use adw::glib::{ParamSpec, ParamSpecObject, Value};
    use std::cell::RefCell;
    use std::sync::LazyLock;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/window.ui")]
    pub struct MonobeanWindow {
        // headbar components
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub base_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub content_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub primary_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub view_switcher: TemplateChild<adw::ViewSwitcher>,
        #[template_child]
        pub search_container: TemplateChild<gtk::Box>,

        // content page
        #[template_child]
        pub hello_page: TemplateChild<HelloPage>,
        // #[template_child]
        // pub mega_tab: TemplateChild<MegaTab>,
        // #[template_child]
        // pub repo_tab: TemplateChild<RepoTab>,
        #[template_child]
        pub code_page: TemplateChild<CodePage>,

        #[template_child]
        pub not_implemented: TemplateChild<NotImplemented>,

        //bottom bar component
        #[template_child]
        pub mega_status_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        pub mega_popup: RefCell<Option<MegaTabWindow>>,

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
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> =
                LazyLock::new(|| vec![ParamSpecObject::builder::<Toast>("toast").build()]);
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

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.toast.replace(Some(Toast::new("")));

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
        let window: MonobeanWindow = glib::Object::new();
        window.set_application(Some(application));
        window.imp().sender.set(sender).unwrap();

        window.setup_headbar_widget();
        window.setup_bottombar_widget();
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
        let code_page = imp.code_page.get();

        imp.hello_page.setup_hello_page(self.sender());
        code_page.setup_code_page(self.sender(), None);

        imp.content_stack
            .connect_visible_child_name_notify(move |stk| {
                if stk.visible_child_name().is_some_and(|name| name == "code") {
                    let file_tree = code_page.imp().file_tree_view.get();
                    CONTEXT.spawn_local_with_priority(Priority::LOW, async move {
                        file_tree.refresh_root().await;
                    });
                }
            });

        // We are developing, so always show hello_page for debug
        let stack = imp.base_stack.clone();
        stack.set_visible_child_name("hello_page");
        let action = Action::ShowHelloPage;
        self.sender().send_blocking(action).unwrap();
    }

    pub fn show_main_page(&self) {
        let stack = self.imp().base_stack.clone();
        let switcher = self.imp().view_switcher.clone();
        switcher.set_visible(true);
        let searcher = self.imp().search_container.clone();
        searcher.set_visible(true);
        stack.set_visible_child_name("main_page");

        // //
        // let content_stack = self.imp().content_stack.get();
        // content_stack.set_visible_child_name("code");

        // 强制刷新文件树
        let code_page = self.imp().code_page.get();
        let file_tree = code_page.imp().file_tree_view.get();
        CONTEXT.spawn_local_with_priority(Priority::LOW, async move {
            file_tree.refresh_root().await;
        });
    }

    pub fn show_hello_page(
        &self,
        name: Option<String>,
        email: Option<String>,
        pgp_generated: bool,
    ) {
        let stack = self.imp().base_stack.clone();
        let page = self.imp().hello_page.clone();
        let switcher = self.imp().view_switcher.clone();
        switcher.set_visible(false);
        let searcher = self.imp().search_container.clone();
        searcher.set_visible(false);
        page.fill_entries(name, email, pgp_generated);
        stack.set_visible_child_name("hello_page");
    }

    fn setup_headbar_widget(&self) {
        let imp = self.imp();
        let prim_btn = imp.primary_menu_button.get();
        let popover = prim_btn.popover().unwrap();
        let popover_menu = popover.downcast::<PopoverMenu>().unwrap();
        let theme = ThemeSelector::new();
        popover_menu.add_child(&theme, "theme");

        // popoverMenu cont find win.(action) change the action group
        let action_group = SimpleActionGroup::new();

        //  style-variant action
        let settings = self.settings().clone();
        let action = SimpleAction::new_stateful(
            "style-variant",
            Some(&String::static_variant_type()),
            &settings.string("style-variant").to_variant(),
        );
        action.connect_activate(move |action, parameter| {
            if let Some(param) = parameter {
                let value = param.get::<String>().unwrap();
                action.set_state(&value.to_variant());
                settings
                    .set_string("style-variant", &value)
                    .expect("Failed to set style-variant in GSettings");
            }
        });

        action_group.add_action(&action);

        popover_menu.insert_action_group("style", Some(&action_group));
    }

    fn setup_bottombar_widget(&self) {
        let imp = self.imp();
        let mega_status_button = imp.mega_status_button.get();
        let status_icon = imp.status_icon.get();
        let status_label = imp.status_label.get();
        mega_status_button.connect_clicked(clone!(
            #[weak(rename_to=window)]
            self,
            #[weak]
            imp,
            move |_| {
                {
                    let mut popup_ref = imp.mega_popup.borrow_mut();
                    if let Some(popup) = popup_ref.as_ref() {
                        if popup.is_visible() {
                            popup.close();
                            popup_ref.take();
                            return;
                        }
                    }
                }

                let popup = MegaTabWindow::new();
                popup.set_transient_for(Some(&window));
                popup.set_modal(false);
                popup.present();
                imp.mega_popup.replace(Some(popup));
            }
        ));

        // mega status bar show
        // todo here produce tons of log ,need to adjust logic of generate log
        let monobean_application: MonobeanApplication =
            self.application().unwrap().downcast().unwrap();
        CONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
            loop {
                let rx = monobean_application.core_status();
                let (core_started, _) = rx.await.unwrap();
                if core_started {
                    status_label.set_label("Mega started");
                    status_icon.set_icon_name(Some("status-normal-icon"));
                    //tracing::debug!("watching mage status----------")
                } else {
                    status_label.set_label("Mega stoped");
                    status_icon.set_icon_name(Some("dialog-warning"));
                    tracing::debug!("watching mage status faild----------")
                }
                glib::timeout_future(std::time::Duration::from_secs(5)).await;
            }
        });
    }

    pub fn add_toast(&self, message: String) {
        let pre = self.property::<Toast>("toast");

        let toast = Toast::builder()
            .title(glib::markup_escape_text(&message))
            .priority(adw::ToastPriority::High)
            .timeout(3)
            .build();
        self.set_property("toast", &toast);
        self.imp().toast_overlay.add_toast(toast);

        // seems that dismiss will clear something used by animation
        // cause adw_animation_skip emit 'done' segfault on closure(https://github.com/gmg137/netease-cloud-music-gtk/issues/202)
        // delay to wait for animation skipped/done
        CONTEXT.spawn_local_with_priority(Priority::DEFAULT_IDLE, async move {
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
            provider.load_from_resource(&format!("{PREFIX}/css/{f}"));
            style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not connect to a display."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        })
        .collect::<Vec<_>>();
}
