use crate::application::Action;
use adw::prelude::*;
use async_channel::Sender;
use gtk::gio::{ListModel, ListStore};
use gtk::glib::property::PropertySet;
use gtk::glib::Enum;
use gtk::glib::{clone, Properties};
use gtk::subclass::prelude::*;
use gtk::{
    glib, prelude::*, BuilderListItemFactory, CompositeTemplate, SignalListItemFactory,
    SingleSelection, TreeListModel,
};
use std::fs::DirEntry;
use std::{cell::OnceCell, path::Path};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

mod imp {
    use std::path::PathBuf;

    use smallvec::SmallVec;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::FileTreeRowData)]
    pub struct FileTreeRowData {
        #[property(name = "label", get, set)]
        pub label: RefCell<String>,
        #[property(name = "path", get, set)]
        pub path: RefCell<PathBuf>,
        #[property(name = "depth", get, set)]
        pub depth: Cell<u8>,
        #[property(name = "file-type", get, set, builder(FileType::File))]
        pub file_type: Cell<FileType>,
        #[property(name = "expanded", get, set)]
        pub expanded: Cell<bool>,
    }

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/file_tree.ui")]
    pub struct FileTreeView {
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/file_row.ui")]
    pub struct FileTreeRow {
        #[template_child]
        pub expander: TemplateChild<gtk::TreeExpander>,
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        pub sender: OnceCell<Sender<Action>>,
        pub bindings: RefCell<SmallVec<[glib::Binding; 4]>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileTreeRowData {
        const NAME: &'static str = "FileTreeRowData";
        type Type = super::FileTreeRowData;
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileTreeView {
        const NAME: &'static str = "FileTreeView";
        type ParentType = gtk::Box;
        type Type = super::FileTreeView;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            // klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileTreeRow {
        const NAME: &'static str = "FileTreeRow";
        type ParentType = gtk::ListBoxRow;
        type Type = super::FileTreeRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileTreeRowData {}

    impl ObjectImpl for FileTreeView {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for FileTreeView {}
    impl BoxImpl for FileTreeView {}

    impl ObjectImpl for FileTreeRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for FileTreeRow {}
    impl ListBoxRowImpl for FileTreeRow {}
}

glib::wrapper! {
    pub struct FileTreeRowData(ObjectSubclass<imp::FileTreeRowData>);
}

glib::wrapper! {
    pub struct FileTreeView(ObjectSubclass<imp::FileTreeView>)
       @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

glib::wrapper! {
    pub struct FileTreeRow(ObjectSubclass<imp::FileTreeRow>)
       @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl FileTreeView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn sender(&self) -> Sender<Action> {
        self.imp().sender.get().unwrap().clone()
    }

    pub fn setup_file_tree(&self, sender: Sender<Action>, mount_point: impl AsRef<Path>) {
        let imp = self.imp();
        imp.sender.set(sender).unwrap();

        let mut root_model = ListStore::new::<FileTreeRowData>();
        let root_items = mount_point
            .as_ref()
            .read_dir()
            .unwrap()
            .into_iter()
            .filter_map(|i| {
                if let Ok(entry) = i {
                    Some(FileTreeRowData::new(0, entry))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        root_model.extend(root_items);

        let model = TreeListModel::new(root_model, false, false, |item| {
            let model = ListStore::new::<FileTreeRowData>();
            let node = item.downcast_ref::<FileTreeRowData>().unwrap();

            if node.file_type() == FileType::Directory {
                let path = node.path();
                let depth = node.depth() + 1;
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        model.append(&FileTreeRowData::new(depth, entry));
                    }
                }
                Some(model.upcast::<ListModel>())
            } else {
                None
            }
        });

        let sender = self.sender();
        let selection = SingleSelection::new(Some(model));
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            tracing::trace!("Setup file_tree item: {:?}", item);
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = FileTreeRow::new(sender.clone());
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_, item| {
            // item: ListItem -(.item)-> TreeListRow -(.item)-> FileTreeRowData
            println!("Bind file_tree item: {:?}", item.type_().name());
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let list_row = list_item.item().and_downcast::<gtk::TreeListRow>().expect("Item is not a TreeListRow");

            let data = list_row
                .item()
                .and_downcast::<FileTreeRowData>()
                .expect("Item is not a FileTreeRowData");
            let row = list_item
                .child()
                .and_downcast::<FileTreeRow>()
                .expect("Child is not a FileTreeRow");
            row.bind(&data);
        });

        factory.connect_unbind(move |_, item| {
            tracing::trace!("Unbind file_tree item: {:?}", item);
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = list_item
                .child()
                .and_downcast::<FileTreeRow>()
                .expect("Child is not a FileTreeRow");
            row.unbind();
        });

        imp.list_view.set_model(Some(&selection));
        imp.list_view.set_factory(Some(&factory));
    }
}

impl FileTreeRow {
    pub fn new(sender: Sender<Action>) -> Self {
        let row: Self = glib::Object::new();
        row.imp().sender.set(sender).unwrap();
        row
    }

    pub fn bind(&self, data: &FileTreeRowData) {
        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();
        let label = imp.label.get();
        let icon = imp.icon.get();
        let expander = imp.expander.get();

        let label_binding = data
            .bind_property("label", &label, "label")
            .flags(glib::BindingFlags::BIDIRECTIONAL)
            .build();
        let icon_binding = data
            .bind_property("file-type", &icon, "icon-name")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, t: FileType| {
                if t.is_dir() {
                    Some("folder-symbolic")
                } else {
                    Some("text-x-generic-symbolic")
                }
            })
            .build();
        let depth_binding = data
            .bind_property("depth", &expander, "indent-for-depth")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_from(|_, depth: u8| Some(depth * 10))
            .build();
        // let expanded_binding = data
        //     .bind_property("expanded", &expander, "expanded")
        //     .flags(glib::BindingFlags::BIDIRECTIONAL)
        //     .build();
        let expandable_binding = data
            .bind_property("file-type", &expander, "hide-expander")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_from(|_, t: FileType| Some(t.is_dir()))
            .build();

        bindings.push(label_binding);
        bindings.push(icon_binding);
        bindings.push(depth_binding);
        bindings.push(expandable_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

impl FileTreeRowData {
    pub fn new(depth: u8, entry: DirEntry) -> Self {
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = if entry.file_type().unwrap().is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        glib::Object::builder()
            .property("label", name)
            .property("path", entry.path())
            .property("depth", depth)
            .property("file-type", file_type)
            .property("expanded", false)
            .build()
    }
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

impl FileType {
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }
}
