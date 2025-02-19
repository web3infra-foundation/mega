use adw::prelude::SettingsExtManual;
use async_channel::Sender;
use gtk::{glib, CompositeTemplate};
use gtk::prelude::{ButtonExt, EditableExt};
use gtk::subclass::prelude::*;
use crate::application::Action;

mod imp {
    use std::cell::{OnceCell, RefCell};
    use adw::gio::Settings;
    use async_channel::Sender;
    use gtk::prelude::WidgetExt;
    use crate::application::Action;
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/hello_page.ui")]
    pub struct HelloPage {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub primary_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub name_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub email_entry: TemplateChild<adw::EntryRow>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HelloPage {
        const NAME: &'static str = "HelloPage";
        type Type = super::HelloPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HelloPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.fill_entries(None, None);
        }
    }

    impl WidgetImpl for HelloPage {}
    impl BoxImpl for HelloPage {}
}

glib::wrapper! {
    pub struct HelloPage(ObjectSubclass<imp::HelloPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl HelloPage {
    pub fn new() -> Self {
        glib::object::Object::new()
    }
    
    pub fn setup_hello_page(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).expect("Hello Page sender can only be set once");
    }
    
    fn setup_action(&self) {
        let sender = self.imp().sender.get().unwrap().clone();
        let back_button = self.imp().back_button.clone();
        back_button.connect_clicked(move |_| {
            let _ = sender.send(Action::ShowMainPage);
        });
    }
    
    pub fn fill_entries(&self, name: Option<String>, email: Option<String>) {
        if let Some(name) = name {
            self.imp().name_entry.set_text(&name);
        }
        if let Some(email) = email {
            self.imp().email_entry.set_text(&email);
        }
    }
}



impl Default for HelloPage {
    fn default() -> Self {
        Self::new()
    }
}
