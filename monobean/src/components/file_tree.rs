use crate::application::Action;
use adw::prelude::*;
use adw::subclass::prelude::BinImpl;
use async_channel::Sender;
use gtk::gio::{ListModel, ListStore};
use gtk::glib::Enum;
use gtk::glib::{clone, Properties};
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, SignalListItemFactory, SingleSelection, TreeListModel};
use smallvec::SmallVec;
use std::cell::{Cell, RefCell};
use std::fs::DirEntry;
use std::path::PathBuf;
use std::{cell::OnceCell, path::Path};

mod imp {
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
        type ParentType = adw::Bin;
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
    impl BinImpl for FileTreeRow {}
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
       @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for FileTreeView {
    fn default() -> Self {
        Self::new()
    }
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

        factory.connect_bind(clone!(
            #[weak]
            selection,
            move |_, item| {
                // item: ListItem -(.item)-> TreeListRow -(.item)-> FileTreeRowData
                tracing::trace!("Bind file_tree item: {:?}", item.type_().name());
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let list_row = list_item
                    .item()
                    .and_downcast::<gtk::TreeListRow>()
                    .expect("Item is not a TreeListRow");

                let data = list_row
                    .item()
                    .and_downcast::<FileTreeRowData>()
                    .expect("Item is not a FileTreeRowData");
                let row = list_item
                    .child()
                    .and_downcast::<FileTreeRow>()
                    .expect("Child is not a FileTreeRow");
                row.imp().expander.set_list_row(Some(&list_row));
                row.bind(&data, &selection);
            }
        ));

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
        row.set_can_focus(true);
        row.set_can_target(true);
        row.set_overflow(gtk::Overflow::Hidden);
        row.set_focus_on_click(true);
        row.imp().sender.set(sender).unwrap();
        row
    }

    pub fn bind(&self, data: &FileTreeRowData, selection: &SingleSelection) {
        let imp = self.imp();
        let label = imp.label.get();
        let icon = imp.icon.get();
        let expander = imp.expander.get();
        let sender = imp.sender.get().unwrap();
        let mut bindings = imp.bindings.borrow_mut();

        tracing::trace!("Bind row name: {:?}", data.label());
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let label_binding = data
            .bind_property("label", &label, "label")
            .sync_create()
            .bidirectional()
            .build();
        let icon_binding = data
            .bind_property("file-type", &icon, "icon-name")
            .sync_create()
            .transform_to(|_, t: FileType| {
                if t.is_dir() {
                    Some("folder-symbolic")
                } else {
                    Some("text-x-generic-symbolic")
                }
            })
            .build();
        let expandable_binding = data
            .bind_property("file-type", &expander, "hide-expander")
            .sync_create()
            .transform_to(|_, t: FileType| Some(!t.is_dir()))
            .build();

        bindings.push(label_binding);
        bindings.push(icon_binding);
        bindings.push(expandable_binding);

        // Connect click handler to expand/collapse directories when clicked
        let gesture = gtk::GestureClick::new();
        gesture.connect_released(clone!(
            #[weak]
            data,
            #[weak]
            expander,
            #[weak]
            selection,
            #[strong]
            sender,
            move |gesture, _, _, _| {
                let list_row = expander.list_row().unwrap();
                let position = list_row.position();
                selection.set_selected(position);

                if data.file_type() == FileType::Directory {
                    let is_expanded = data.expanded();
                    data.set_expanded(!is_expanded);

                    if let Some(list_row) = expander.list_row() {
                        list_row.set_expanded(!is_expanded);
                    }
                } else {
                    // Handle file click - could send an action to open the file
                    let path = data.path();
                    let _ = sender.try_send(Action::OpenEditorOn(path.to_path_buf()));
                }
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        ));

        self.add_controller(gesture);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

impl FileTreeRowData {
    pub fn new(depth: u8, entry: DirEntry) -> Self {
        let name = entry
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
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
#[derive(Default)]
pub enum FileType {
    #[default]
    File = 0,
    Directory,
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }
}
