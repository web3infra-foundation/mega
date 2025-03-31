use std::ops::Deref;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio::FileInfo;
use gtk::glib::value::FromValue;
use gtk::glib::{self, ParamSpec, ParamSpecUInt, ParamSpecUIntBuilder, Value, ValueDelegate};
use gtk::glib::translate::{FromGlib, IntoGlib};
use gtk::glib::{ParamSpecEnum};
use gtk::prelude::*;

mod imp {
    use std::cell::RefCell;

    use super::*;
    use gtk::glib::Properties;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::FileRowObject)]
    pub struct FileRowObject {
        #[property(name = "depth", get, set, type = u8, member = depth)]
        #[property(name = "label", get, set, type = String, member = label)]
        pub data: RefCell<FileRowData>,

        #[property(name = "file-type", get, set, type = FileType)]
        pub file_type: FileType,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileRowObject {
        const NAME: &'static str = "FileObject";
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

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    File = 0,
    Directory,
}

impl Default for FileType {
    fn default() -> Self {
        FileType::File
    }
}

impl From<u32> for FileType {
    fn from(value: u32) -> Self {
        match value {
            0 => FileType::File,
            1 => FileType::Directory,
            _ => FileType::File,
        }
    }
}

impl From<&FileType> for u32 {
    fn from(value: &FileType) -> u32 {
        match value {
            FileType::File => 0,
            FileType::Directory => 1,
        }
    }
}

impl ToValue for FileType {
    fn to_value(&self) -> Value {
        let value = Value::from(Into::<u32>::into(self));
        value
    }

    fn value_type(&self) -> glib::Type {
        glib::Type::U32
    }
}
impl HasParamSpec for FileType {
    type ParamSpec = ParamSpecUInt;

    type SetValue = u32;

    type BuilderFn = fn(&str) -> ParamSpecUIntBuilder;

    fn param_spec_builder() -> Self::BuilderFn {
        Self::ParamSpec::builder
    }
}

unsafe impl FromValue<'_> for FileType {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        let value = value.get::<u32>().expect("Wrong type");
        value.into()
    }
}
