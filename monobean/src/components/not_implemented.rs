use adw::gio;
use gtk::glib;
use gtk::CompositeTemplate;

use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/not_implemented.ui")]
    pub struct NotImplemented;

    #[glib::object_subclass]
    impl ObjectSubclass for NotImplemented {
        const NAME: &'static str = "NotImplemented";
        type Type = super::NotImplemented;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NotImplemented {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for NotImplemented {}
    impl BoxImpl for NotImplemented {}
}

glib::wrapper! {
    pub struct NotImplemented(ObjectSubclass<imp::NotImplemented>)
        @extends gtk::Widget, gtk::Box,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for NotImplemented {
    fn default() -> Self {
        Self::new()
    }
}

impl NotImplemented {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
