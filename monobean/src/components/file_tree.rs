use crate::application::Action;
use crate::CONTEXT;
use adw::prelude::*;
use adw::subclass::prelude::BinImpl;
use async_channel::Sender;
use gtk::gio::{ListModel, ListStore};
use gtk::glib::Enum;
use gtk::glib::{clone, Properties};
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, SignalListItemFactory, SingleSelection, TreeListModel};
use mercury::internal::object::tree::{TreeItem, TreeItemMode};
use smallvec::SmallVec;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::{cell::OnceCell, path::Path};


mod imp {
    use std::rc::Rc;

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
        #[property(name = "hash", get, set)]
        pub hash: RefCell<String>,
    }

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/file_tree.ui")]
    pub struct FileTreeView {
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,

        pub sender: OnceCell<Sender<Action>>,
        pub root_store: Rc<RefCell<Option<ListStore>>>,
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
        // #[template_child]
        // pub tree_lines: TemplateChild<gtk::DrawingArea>,

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

    pub fn setup_file_tree(&self, sender: Sender<Action>) {
        let imp = self.imp();
        imp.sender.set(sender.clone()).unwrap();

        // At this time, mega core is not initialized,
        // we can not get the root directory from mega core.
        let root = ListStore::new::<FileTreeRowData>();

        *self.imp().root_store.borrow_mut() = Some(root.clone());

        let model = TreeListModel::new(root, false, false, move |item| {
            let model = ListStore::new::<FileTreeRowData>();
            let node = item.downcast_ref::<FileTreeRowData>().unwrap();

            if node.file_type() == FileType::Directory {
                let path = node.path();
                let depth = node.depth() + 1;
                let sender = sender.clone();
                let mut ref_model = model.clone();
                CONTEXT.spawn_local(async move {
                    let (dirs, files) = Self::load_directory(sender, Some(path), depth).await;
                    ref_model.extend(dirs);
                    ref_model.extend(files);
                });
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

    pub async fn refresh_root(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let mount_point = Some(PathBuf::from("/"));
        let (root_dirs, root_files) = Self::load_directory(sender, mount_point, 0).await;

        if let Some(ref mut root_store) = *imp.root_store.borrow_mut() {
            root_store.remove_all();
            root_store.extend(root_dirs);
            root_store.extend(root_files);
        }
    }

    async fn load_directory(
        sender: Sender<Action>,
        path: Option<impl AsRef<Path>>,
        depth: u8,
    ) -> (Vec<FileTreeRowData>, Vec<FileTreeRowData>) {
        let path = path.map(|inner| inner.as_ref().to_path_buf());
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        sender
            .send(Action::MegaCore(
                crate::core::mega_core::MegaCommands::LoadFileTree {
                    chan: tx,
                    path: path.clone(),
                },
            ))
            .await
            .unwrap();

        if let Ok(Ok(tree)) = rx.await {
            let path = path.unwrap_or(PathBuf::from("/"));
            for entry in tree.tree_items {
                match entry.mode {
                    TreeItemMode::Blob | TreeItemMode::BlobExecutable => {
                        files.push(FileTreeRowData::new(depth, &path, entry));
                    }
                    TreeItemMode::Tree => {
                        dirs.push(FileTreeRowData::new(depth, &path, entry));
                    }
                    _ => {}
                }
            }
        } else {
            tracing::error!("Failed to load directory: {:?}", path);
        }
        (dirs, files)
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
        //let tree_line = imp.tree_lines.get();
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
            .bind_property("path", &icon, "icon-name")
            .sync_create()
            .transform_to(move |_, t: glib::GString| {

                let path = Path::new(t.as_str());

                match path.file_name().and_then(|name| name.to_str()) {
                    Some(name) if  name.eq(".idea")||name.eq(".vscode") || !name.contains(".")
                    => Some("folder-symbolic"),

                    // .gitignore is not extension but file name
                    Some(name) if [".gitignore", ".gitattributes", ".gitmodules"].contains(&name) => {
                        Some("monobean-gitFile-symbolic")
                    }
                    _ => match path.extension().and_then(|ext| ext.to_str()) {
                        Some("toml") => Some("monobean-settingFile-symbolic"),
                        Some("md") => Some("monobean-markdown-symbolic"),
                        Some("rs") => Some("monobean-rust-symbolic"),
                        Some("png") | Some("svg") | Some("jpg") => Some("monobean-picture-symbolic"),
                        Some("xml") | Some("ui") => Some("monobean-rss-symbolic"),
                        Some("css") => Some("monobean-css-symbolic"),
                        Some("json") => Some("monobean-json-symbolic"),
                        _ => Some("text-x-generic"),
                    },
                }
            })
            .build();

        
        // let depth = data.depth(); 
        // tree_line.set_draw_func(move |_area, ctx, _width, height| {
        //     let line_spacing = 12.0; 
        //     let line_offset = 6.0;   
        // 
        //     ctx.set_source_rgba(0.7, 0.7, 0.7, 1.0); 
        //     ctx.set_line_width(1.0);
        //     ctx.set_dash(&[2.0, 2.0], 0.0);
        // 
        //     for i in 0..depth {
        //         let x = i as f64 * line_spacing + line_offset;
        // 
        //         // 当前层
        //         if i == depth - 1 {
        //             if !is_last_child {
        //                 // 当前节点不是最后一个 → 画半根线（中间到底部）
        //                 ctx.move_to(x, height as f64 / 2.0);
        //                 ctx.line_to(x, height as f64);
        //             }
        //         } else {
        //             // 上层：始终画整条线（贯穿）
        //             ctx.move_to(x, 0.0);
        //             ctx.line_to(x, height as f64);
        //         }
        //     }
        // 
        //     ctx.stroke().unwrap();
        // });






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
                    let hash = data.hash();
                    let name = data.label();
                    let path = data.path();
                    let _ = sender.try_send(Action::OpenEditorOn { hash, name ,path});
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
    pub fn new(depth: u8, parent: impl AsRef<Path>, entry: TreeItem) -> Self {
        let name = entry.name;
        let file_type = match entry.mode {
            TreeItemMode::Tree => FileType::Directory,
            _ => FileType::File,
        };

        glib::Object::builder()
            .property("path", parent.as_ref().join(&name))
            .property("label", name)
            .property("depth", depth)
            .property("file-type", file_type)
            .property("expanded", false)
            .property("hash", entry.id.to_string())
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
