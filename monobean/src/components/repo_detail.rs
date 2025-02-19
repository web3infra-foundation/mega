use gtk::{glib, CompositeTemplate};

use gtk::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/repo_detail.ui")]
    pub struct RepoDetail {
        // #[template_child]
        // pub repo_name: TemplateChild<gtk::Label>,
        // #[template_child]
        // pub description: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoDetail {
        const NAME: &'static str = "RepoDetail";
        type Type = super::RepoDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoDetail {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for RepoDetail {}
    impl BoxImpl for RepoDetail {}
}

glib::wrapper! {
    pub struct RepoDetail(ObjectSubclass<imp::RepoDetail>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl RepoDetail {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for RepoDetail {
    fn default() -> Self {
        Self::new()
    }
}
