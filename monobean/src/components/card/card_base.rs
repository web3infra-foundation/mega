use gtk::{glib, CompositeTemplate};

use gtk::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/card_base.ui")]
    pub struct CardBase {

    }

    #[glib::object_subclass]
    impl ObjectSubclass for CardBase {
        const NAME: &'static str = "CardBase";
        type Type = super::CardBase;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CardBase {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for CardBase {}
    impl BoxImpl for CardBase {}
}

glib::wrapper! {
    pub struct CardBase(ObjectSubclass<imp::CardBase>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl CardBase {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for CardBase {
    fn default() -> Self {
        Self::new()
    }
}
