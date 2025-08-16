use gtk::glib;

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
 

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HistoryItem)]
    pub struct HistoryItem {
        #[property(get, set)]
        pub id: RefCell<String>,
        #[property(get, set)]
        pub tree_id: RefCell<String>,
        #[property(get, set)]
        pub text: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HistoryItem {
        const NAME: &'static str = "HistoryItem";
        type Type = super::HistoryItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for HistoryItem {}
}

glib::wrapper! {
    pub struct HistoryItem(ObjectSubclass<imp::HistoryItem>);
}

impl HistoryItem {
    pub fn new(id: &str,tree_id: &str,text: &str) -> Self {
        glib::Object::builder()
            .property("text", text)
            .property("id", id)
            .property("tree_id", tree_id)
            .build()
    }
}
