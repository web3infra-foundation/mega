use async_channel::Sender;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

use crate::application::Action;

mod imp {
    use tokio::sync::OnceCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/code_page.ui")]
    pub struct CodePage {
        // #[template_child]
        // pub searchbar: TemplateChild<gtk::SearchBar>,
        // #[template_child]
        // pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub listview: TemplateChild<gtk::ListView>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CodePage {
        const NAME: &'static str = "CodePage";
        type Type = super::CodePage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CodePage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for CodePage {}
    impl BoxImpl for CodePage {}
}

glib::wrapper! {
    pub struct CodePage(ObjectSubclass<imp::CodePage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for CodePage {
    fn default() -> Self {
        Self::new()
    }
}

impl CodePage {
    pub fn new() -> Self {
        glib::object::Object::new()
    }

    pub fn setup_code_page(&self, sender: Sender<Action>) {
        self.imp()
            .sender
            .set(sender)
            .expect("Code Page sender can only be set once");
        // self.setup_action();
    }
}
