use adw::NavigationSplitView;
use gtk::{glib, CompositeTemplate};
use gtk::{Box, Stack};

use gtk::subclass::prelude::*;

mod imp {
    use crate::components::repo_detail::RepoDetail;

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/repo_tab.ui")]
    pub struct RepoTab {
        #[template_child]
        pub nav_view: TemplateChild<NavigationSplitView>,
        #[template_child]
        pub info_stack: TemplateChild<Stack>,
        #[template_child]
        pub repo_detail: TemplateChild<RepoDetail>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTab {
        const NAME: &'static str = "RepoTab";
        type Type = super::RepoTab;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTab {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for RepoTab {}
    impl BoxImpl for RepoTab {}
}

glib::wrapper! {
    pub struct RepoTab(ObjectSubclass<imp::RepoTab>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl RepoTab {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

#[gtk::template_callbacks]
impl RepoTab {
    #[template_callback]
    pub fn notify_visible_child_cb(&self) {
        // let imp = self.imp();
        // let stack = imp.stack.get();
        // let label = imp.label_title.get();
        // if let Some(visible_child_name) = stack.visible_child_name() {
        //     let mut info_stack = LinkedList::new();
        //     if let Ok(sc) = imp.stack_child.lock() {
        //         info_stack = (*sc).clone();
        //     }
        //     if let Some(child) = info_stack.back() {
        //         if visible_child_name == child.0 {
        //             return;
        //         }
        //     }
        //     if info_stack.len() == 1 {
        //         if visible_child_name == "discover"
        //             || visible_child_name == "toplist"
        //             || visible_child_name == "my"
        //         {
        //             if let Ok(mut sc) = imp.stack_child.lock() {
        //                 sc.pop_back();
        //                 sc.push_back((visible_child_name.to_string(), "".to_owned()));
        //             }
        //         } else if let Ok(mut sc) = imp.stack_child.lock() {
        //             sc.push_back((visible_child_name.to_string(), label.text().to_string()));
        //         }
        //     } else if visible_child_name == "discover"
        //         || visible_child_name == "toplist"
        //         || visible_child_name == "my"
        //     {
        //         if let Ok(mut sc) = imp.stack_child.lock() {
        //             sc.clear();
        //             sc.push_back((visible_child_name.to_string(), "".to_owned()));
        //         }
        //     } else if let Ok(mut sc) = imp.stack_child.lock() {
        //         sc.push_back((visible_child_name.to_string(), label.text().to_string()));
        //     }
        // }
    }
}

impl Default for RepoTab {
    fn default() -> Self {
        Self::new()
    }
}
