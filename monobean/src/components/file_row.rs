use gtk::glib;
use gtk::{self, CompositeTemplate};
use gtk::glib::subclass::InitializingObject;

use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/file_row.ui")]
    pub struct FileRow {
        #[template_child]
        pub expander: TemplateChild<gtk::TreeExpander>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileRow {
        const NAME: &'static str = "FileRow";
        type Type = super::FileRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FileRow {}
    impl WidgetImpl for FileRow {}
    impl BoxImpl for FileRow {}
}

glib::wrapper! {
    pub struct FileRow(ObjectSubclass<imp::FileRow>)
        @extends gtk::Widget, gtk::Box;
}

impl FileRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_item<O: IsA<glib::Object>>(&self, item: &O) {
        self.set_property("item", item);
    }
}
