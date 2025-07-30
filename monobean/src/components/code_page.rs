use crate::application::Action;
use crate::components::history_list::HistoryItem;
use crate::core::mega_core::MegaCommands;
use crate::CONTEXT;
use adw::gdk;
use adw::gio::ListStore;
use async_channel::Sender;
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
        let list_view = self.imp().history_listview.clone();

        // 创建数据模型
        let store = ListStore::new::<HistoryItem>();

        for i in 1..=20 {
            store.append(&HistoryItem::new(&format!("commit信息 \n 提交人:{i}, sha")));
        }
        let selection_model = SingleSelection::new(Some(store));

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

            label.set_label(&obj.text()); // 获取属性
        });

        list_view.set_model(Some(&selection_model));
        list_view.set_factory(Some(&factory));
    }

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

        // 设置 Popover 在按钮左侧弹出
        history_popover.set_position(gtk::PositionType::Left);
        history_popover.set_has_arrow(false);
        history_popover.set_parent(&history_btn);
        history_popover.set_autohide(false);

        // 按钮点击处理 - 确保位置正确
        history_btn.connect_clicked(clone!(
            #[weak]
            history_popover,
            move |_| {
                if history_popover.is_visible() {
                    history_popover.popdown();
                } else {
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
