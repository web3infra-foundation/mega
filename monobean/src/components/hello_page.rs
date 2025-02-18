use gtk::{glib, CompositeTemplate};

use gtk::subclass::prelude::*;

mod imp {
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
        glib::Object::new()
    }
}

impl Default for HelloPage {
    fn default() -> Self {
        Self::new()
    }
}
