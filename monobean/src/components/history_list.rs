use glib::{Object, Value};
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    
    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HistoryItem)]
    pub struct HistoryItem {
        
        #[property(get, set)]
        pub text: RefCell<String >,
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
    pub fn new(text: &str) -> Self {
        glib::Object::builder()
            .property("text", text)
            .build()
    }
}

