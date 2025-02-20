use adw::gio;
use gtk::glib;
use gtk::CompositeTemplate;

use adw::prelude::*;
use adw::subclass::prelude::*;

glib::wrapper! {
    pub struct MegaTab(ObjectSubclass<imp::MegaTab>)
        @extends gtk::Widget, gtk::Box,
        @implements gio::ActionGroup, gio::ActionMap;
}

mod imp {
    use gtk::Button;

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/mega_tab.ui")]
    pub struct MegaTab {
        #[template_child]
        pub toggle_mega: TemplateChild<Button>,
        #[template_child]
        pub toggle_fuse: TemplateChild<Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MegaTab {
        const NAME: &'static str = "MegaTab";
        type Type = super::MegaTab;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MegaTab {
        fn constructed(&self) {
            const DESTRUCTIVE: &str = "destructive-action";
            const SUGGESTED: &str = "suggested-action";
            self.parent_constructed();

            self.toggle_mega.connect_clicked(|t| {
                if t.has_css_class(DESTRUCTIVE) {
                    t.remove_css_class(DESTRUCTIVE);
                    t.add_css_class(SUGGESTED);
                    t.set_label("_Start Mega");
                } else {
                    t.remove_css_class(SUGGESTED);
                    t.add_css_class(DESTRUCTIVE);
                    t.set_label("_Stop Mega");
                }
            });

            self.toggle_fuse.connect_clicked(|t| {
                if t.has_css_class(DESTRUCTIVE) {
                    t.remove_css_class(DESTRUCTIVE);
                    t.add_css_class(SUGGESTED);
                    t.set_label("_Start Fuse");
                } else {
                    t.remove_css_class(SUGGESTED);
                    t.add_css_class(DESTRUCTIVE);
                    t.set_label("_Stop Fuse");
                }
            });
        }
    }
    impl WidgetImpl for MegaTab {}
    impl BoxImpl for MegaTab {}
}

impl MegaTab {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for MegaTab {
    fn default() -> Self {
        Self::new()
    }
}
