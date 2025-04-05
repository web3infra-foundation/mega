use std::path::{Path, PathBuf};

use async_channel::Sender;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use scv::{prelude::*, Buffer, StyleSchemeChooser};

use crate::application::Action;

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
        pub paned: TemplateChild<gtk::Paned>,
        #[template_child]
        pub file_tree_view: TemplateChild<FileTreeView>,
        #[template_child]
        pub code_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub empty_view: TemplateChild<gtk::Box>,
        #[template_child]
        pub source_view: TemplateChild<scv::View>,

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
        self.setup_source_view(opened_file);
        self.setup_file_tree();
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

        let file_tree_view = self.imp().file_tree_view.get();
        file_tree_view.setup_file_tree(
            imp.sender.get().unwrap().clone(),
            PathBuf::from("E:/Projects/mega/"),
        );
    }

    fn setup_source_view(&self, opened_file: Option<&Path>) {
        let imp = self.imp();
        let source_view = imp.source_view.get();
        source_view.set_accepts_tab(true);

        match opened_file {
            Some(path) => {
                self.show_editor(path);
            }
            None => {
                self.hide_editor();
            }
        }
    }

    pub fn show_editor(&self, path: impl AsRef<Path>) {
        let imp = self.imp();
        let path = path.as_ref();
        if !path.exists() || !path.is_file() {
            tracing::warn!("file not exists: {:?}", path);
            return;
        }

        let buf = Buffer::new(None);
        let file_name = path.file_name().unwrap().to_str();
        let language_manager = scv::LanguageManager::new();
        let file = adw::gio::File::for_path(path);
        let file = scv::File::builder().location(&file).build();
        let loader = scv::FileLoader::new(&buf, &file);

        buf.set_highlight_syntax(true);
        if let Some(ref language) = language_manager.guess_language(file_name, None) {
            tracing::debug!("Guessed language: {:?}", language);
            buf.set_language(Some(language));
        }
        if let Some(ref scheme) = scv::StyleSchemeManager::new().scheme("Adwaita-dark") {
            buf.set_style_scheme(Some(scheme));
        }

        loader.load_async_with_callback(
            glib::Priority::default(),
            adw::gio::Cancellable::NONE,
            move |current_num_bytes, total_num_bytes| {
                tracing::debug!(
                    "loading: {:?}",
                    (current_num_bytes as f32 / total_num_bytes as f32) * 100f32,
                );
            },
            |res| {
                tracing::debug!("loaded: {}", res.is_ok());
            },
        );

        imp.source_view.set_buffer(Some(&buf));
        imp.code_stack.set_visible_child_name("source_view");
    }

    pub fn hide_editor(&self) {
        let imp = self.imp();
        imp.code_stack.set_visible_child_name("empty_view");
    }
}
