use std::ops::Deref;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio::FileInfo;
use gtk::glib::enums::EnumTypeChecker;
use gtk::glib::property::Property;
use gtk::glib::value::FromValue;
use gtk::glib::{self, Enum, ParamSpec, ParamSpecUInt, ParamSpecUIntBuilder, Value, ValueDelegate};
use gtk::glib::translate::{FromGlib, IntoGlib};
use gtk::glib::{ParamSpecEnum};
use gtk::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::glib::Properties;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::FileRowObject)]
    pub struct FileRowObject {
        #[property(name = "depth", get, set, type = u8, member = depth)]
        #[property(name = "label", get, set, type = String, member = label)]
        pub data: RefCell<FileRowData>,

        #[property(get, set, builder(FileType::File))]
        pub file_type: Rc<RefCell<FileType>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileRowObject {
        const NAME: &'static str = "FileRowObject";
        type Type = super::FileRowObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileRowObject {}
}

glib::wrapper! {
    pub struct FileRowObject(ObjectSubclass<imp::FileRowObject>);
}

impl FileRowObject {
    pub fn new(depth: u8, label: String, file_type: FileType) -> Self {
        glib::Object::builder()
            .property("depth", depth)
            .property("label", label)
            .property("file-type", file_type)
            .build()
    }
}

#[derive(Default, Debug)]
pub struct FileRowData {
    pub depth: u8,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
#[enum_type(name = "FileType")]
pub enum FileType {
    File = 0,
    Directory,
}

impl Default for FileType {
    fn default() -> Self {
        FileType::File
    }
}
