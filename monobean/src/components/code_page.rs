use crate::application::Action;
use crate::components::history_list::HistoryItem;
use crate::core::mega_core::MegaCommands;
use crate::CONTEXT;
use adw::gdk;
use adw::gio::ListStore;
use async_channel::Sender;
use chrono::{DateTime, Utc};
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label, ListItem, SignalListItemFactory, SingleSelection};
use scv::{prelude::*, Buffer};
use std::path::{Path, PathBuf};
use tokio::sync::oneshot;

mod imp {
    use adw::subclass::prelude::BinImpl;
    use tokio::sync::OnceCell;

    use crate::components::file_tree::FileTreeView;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/code_page.ui")]
    pub struct CodePage {
        // #[template_child]
        // pub searchbar: TemplateChild<gtk::SearchBar>,
        // #[template_child]
        // pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub file_path_label: TemplateChild<Label>,
        #[template_child]
        pub paned: TemplateChild<gtk::Paned>,

        #[template_child]
        pub file_tree_view: TemplateChild<FileTreeView>,
        #[template_child]
        pub code_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub empty_view: TemplateChild<gtk::Box>,
        #[template_child]
        pub source_view: TemplateChild<scv::View>,
        #[template_child]
        pub url_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub history_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub url_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub history_popover: TemplateChild<gtk::Popover>,
        #[template_child(id = "history_listview")]
        pub history_listview: TemplateChild<gtk::ListView>,

        pub cur_file_path: std::sync::RwLock<PathBuf>,
        pub history_selection_signal: std::sync::RwLock<Option<glib::SignalHandlerId>>,
        pub history_store: std::sync::RwLock<Option<glib::WeakRef<ListStore>>>,
        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CodePage {
        const NAME: &'static str = "CodePage";
        type Type = super::CodePage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CodePage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for CodePage {}
    impl BinImpl for CodePage {}
}

glib::wrapper! {
    pub struct CodePage(ObjectSubclass<imp::CodePage>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for CodePage {
    fn default() -> Self {
        Self::new()
    }
}

impl CodePage {
    pub fn new() -> Self {
        glib::object::Object::new()
    }

    pub fn setup_code_page(&self, sender: Sender<Action>, opened_file: Option<&Path>) {
        self.imp()
            .sender
            .set(sender)
            .expect("Code Page sender can only be set once");
        // self.setup_action();

        self.setup_paned();
        self.setup_button();
        self.setup_history_list();
        self.setup_source_view(opened_file);
        self.setup_file_tree();
    }

    fn setup_history_list(&self) {
        let imp = self.imp();
        let list_view = self.imp().history_listview.clone();

        // 初始化列表视图组件
        let store = ListStore::new::<HistoryItem>();
        let selection_model = SingleSelection::new(Some(store.clone()));
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let label = Label::new(None);
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        factory.connect_bind(move |_, list_item| {
            let obj = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<HistoryItem>()
                .expect("Model should be HistoryItem");

            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<Label>()
                .expect("Child widget must be a Label");

            label.set_label(&obj.text());
        });

        list_view.set_model(Some(&selection_model));
        list_view.set_factory(Some(&factory));

        {
            let mut store_guard = imp.history_store.write().unwrap();
            *store_guard = Some(store.downgrade());
        }

        {
            let mut signal_guard = imp.history_selection_signal.write().unwrap();
            if let Some(handler_id) = signal_guard.take() {
                list_view.disconnect(handler_id);
            }
        }

        let selection_model_clone = selection_model.clone();
        let sender = imp.sender.get().unwrap().clone();
        let page_weak = self.downgrade();

        let handler_id = list_view.connect_activate(move |_list_view, pos| {
            tracing::debug!("##############点击了第 {} 项", pos);
            selection_model_clone.set_selected(pos);
            if let Some(item) = selection_model_clone.item(pos) {
                if let Ok(history_item) = item.downcast::<HistoryItem>() {
                    let tree_id = history_item.tree_id();
                    let file_path = history_item.file_path();
                    let sender = sender.clone();
                    let page_weak = page_weak.clone();

                    CONTEXT.spawn_local(async move {
                        let (tx, rx) = oneshot::channel();
                        sender
                            .send(Action::MegaCore(MegaCommands::GetHistoryBlobId {
                                chan: tx,
                                tree_id,
                                path: file_path.clone(),
                            }))
                            .await
                            .unwrap();

                        match rx.await {
                            Ok(Ok(blob_id)) => {
                                let path = PathBuf::from(file_path);
                                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                    if let Some(page) = page_weak.upgrade() {
                                        page.show_editor_on(blob_id, file_name.to_string(), path);
                                    } else {
                                        tracing::error!("Failed to upgrade weak reference to page");
                                    }
                                } else {
                                    tracing::error!("Failed to extract file name from path");
                                }
                            }
                            Ok(Err(e)) => {
                                tracing::error!("Failed to get history blob: {:?}", e);
                            }
                            Err(e) => {
                                tracing::error!("Channel error: {:?}", e);
                            }
                        }
                    });
                }
            }
        });

        // 保存信号连接ID
        {
            let mut signal_guard = imp.history_selection_signal.write().unwrap();
            *signal_guard = Some(handler_id);
        }
    }

    fn update_history_list(&self) {
        let imp = self.imp();
        let file_path = imp.cur_file_path.read().unwrap().clone();
        tracing::info!("file_path: {:?}", file_path);

        // 获取store
        let store_ref = imp.history_store.read().unwrap();
        let store = store_ref.as_ref().and_then(|weak| weak.upgrade()).unwrap();

        if file_path.as_os_str().is_empty() {
            store.append(&HistoryItem::new("", "", "", "尚未打开文件"));
            return;
        }

        // 获取列表视图内容
        let sender = imp.sender.get().unwrap().clone();
        let sender_clone = sender.clone();
        let file_path_clone = file_path.to_str().unwrap().to_string();
        CONTEXT.spawn_local(clone!(
            #[weak]
            store,
            async move {
                let (tx, rx) = oneshot::channel();
                sender_clone
                    .send(Action::MegaCore(MegaCommands::GetPathHistory {
                        chan: tx,
                        path: file_path_clone.clone(),
                    }))
                    .await
                    .unwrap();

                match rx.await.unwrap() {
                    Ok(commits) => {
                        //tracing::debug!("Received commit history{:?}",commits);
                        store.remove_all();
                        for commit in commits {
                            let full_message = commit.message;
                            let parts: Vec<&str> = full_message.split("\n\n").collect();
                            let main_message = parts.last().unwrap_or(&"");
                            store.append(&HistoryItem::new(
                                &commit.id._to_string(),
                                &commit.tree_id.to_string(),
                                &file_path_clone,
                                &format!(
                                    "Commit:{}\nAuthor: {}\n message: {}  Date: {}",
                                    commit.id,
                                    commit.author.name,
                                    main_message,
                                    //commit.committer.timestamp.to_string()
                                    DateTime::<Utc>::from_timestamp(
                                        commit.committer.timestamp as i64,
                                        0
                                    )
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                    .unwrap_or_else(|| "Invalid Date".to_string())
                                ),
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::error!("get history commits error: {:?}", e);
                        return;
                    }
                }
            }
        ));
    }

    // sidebar button
    fn setup_button(&self) {
        let imp = self.imp();

        let url_btn = imp.url_btn.get();
        //url_btn.set_child(Some(&image));

        let url_popover = imp.url_popover.get();
        url_popover.set_pointing_to(None);
        url_popover.set_has_arrow(false);
        url_popover.set_parent(&url_btn);
        url_popover.set_autohide(true);
        url_popover.set_position(gtk::PositionType::Left);

        // 弹出时设置一次
        url_btn.connect_clicked(clone!(
            #[weak]
            url_popover,
            move |_| {
                if url_popover.is_visible() {
                    url_popover.popdown();
                } else {
                    url_popover.popup();
                }
            }
        ));

        let history_btn = imp.history_btn.get();
        let history_popover = imp.history_popover.get();

        // set Popover
        history_popover.set_position(gtk::PositionType::Left);
        history_popover.set_has_arrow(false);
        history_popover.set_parent(&history_btn);
        history_popover.set_autohide(true);

        //let page_weak = self.downgrade();
        history_btn.connect_clicked(clone!(
            #[weak(rename_to=page)]
            self,
            #[weak]
            history_popover,
            move |_| {
                if history_popover.is_visible() {
                    history_popover.popdown();
                } else {
                    page.update_history_list();
                    let rect = gdk::Rectangle::new(-5, 200, 100, 30);

                    history_popover.set_pointing_to(Some(&rect));
                    history_popover.popup();
                }
            }
        ));
    }

    fn setup_paned(&self) {
        let imp = self.imp();
        let paned = imp.paned.get();
        let file_tree = imp.file_tree_view.get();
        let code_stack = imp.code_stack.get();

        paned.set_shrink_start_child(false);
        paned.set_shrink_end_child(false);

        file_tree.set_size_request(50, -1);
        code_stack.set_size_request(50, -1);
    }

    fn setup_file_tree(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let file_tree_view = imp.file_tree_view.get();

        file_tree_view.setup_file_tree(sender);
    }

    fn setup_source_view(&self, opened_file: Option<&Path>) {
        let imp = self.imp();
        let source_view = imp.source_view.get();
        source_view.set_can_focus(false);
        source_view.set_editable(false);

        match opened_file {
            Some(_path) => {
                // self.show_editor_on(path);
            }
            None => {
                self.hide_editor();
            }
        }
    }

    pub fn show_editor_on(&self, hash: String, name: String, path: PathBuf) {
        let imp = self.imp();

        tracing::debug!("show_editor_on: path: {:?}", path);

        {
            let mut cur_path = imp.cur_file_path.write().unwrap();
            let mut git_path = path.to_string_lossy().replace('\\', "/");
            if !git_path.starts_with('/') {
                git_path = format!("/{git_path}");
            }
            *cur_path = git_path.parse().unwrap();
        }

        imp.file_path_label
            .set_text(&path.to_str().unwrap().replace(['/', '\\'], " > "));

        let sender = imp.sender.get().unwrap().clone();

        CONTEXT.spawn_local(clone!(
            #[weak(rename_to=page)]
            self,
            async move {
                let buf = Buffer::new(None);
                let language_manager = scv::LanguageManager::new();
                let (tx, rx) = oneshot::channel();

                sender
                    .send(Action::MegaCore(MegaCommands::LoadFileContent {
                        chan: tx,
                        id: hash,
                    }))
                    .await
                    .unwrap();

                match rx.await.unwrap() {
                    Ok(content) => {
                        buf.set_text(content.as_str());
                    }
                    Err(e) => {
                        tracing::error!("load file content error: {:?}", e);
                        return;
                    }
                }

                buf.set_highlight_syntax(true);
                if let Some(ref language) = language_manager.guess_language(Some(name), None) {
                    tracing::debug!("Guessed language: {:?}", language);
                    buf.set_language(Some(language));
                }

                let buf_weak = buf.downgrade();
                let scheme_manager = scv::StyleSchemeManager::new();
                let style_manager = adw::StyleManager::default();
                // 连接主题变化通知
                style_manager.connect_notify_local(
                    Some("color-scheme"),
                    move |style_manager, _| {
                        if let Some(buffer) = buf_weak.upgrade() {
                            let scheme_name = match style_manager.color_scheme() {
                                adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => {
                                    "Adwaita-dark"
                                }
                                adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => {
                                    "Adwaita"
                                }
                                _ => {
                                    // Default 跟随系统
                                    if style_manager.is_dark() {
                                        "Adwaita-dark"
                                    } else {
                                        "Adwaita"
                                    }
                                }
                            };

                            if let Some(scheme) = scheme_manager.scheme(scheme_name) {
                                buffer.set_style_scheme(Some(&scheme));
                            }
                        }
                    },
                );

                let imp = page.imp();
                imp.source_view.set_buffer(Some(&buf));
                imp.code_stack.set_visible_child_name("source_view");
            }
        ));
    }

    pub fn hide_editor(&self) {
        let imp = self.imp();
        imp.code_stack.set_visible_child_name("empty_view");
    }
}
