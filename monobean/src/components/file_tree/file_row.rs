use gtk::glib::Object;
use gtk::{prelude::*, SignalListItemFactory};
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

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
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FileRow {
        fn constructed(&self) {
           self.parent_constructed();

           let obj = self.obj();
        }
    }
    impl WidgetImpl for FileRow {}
    impl BoxImpl for FileRow {}
}

glib::wrapper! {
    pub struct FileRow(ObjectSubclass<imp::FileRow>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for FileRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FileRow {
    pub fn new() -> Self {
        Self::default()
    }
}
