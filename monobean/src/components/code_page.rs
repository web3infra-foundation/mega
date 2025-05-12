use std::path::Path;

use async_channel::Sender;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use scv::{prelude::*, Buffer};
use tokio::sync::oneshot;

use crate::application::Action;
use crate::core::mega_core::MegaCommands;
use crate::CONTEXT;

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

    pub fn show_editor_on(&self, hash: String, name: String) {
        let imp = self.imp();
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
                if let Some(ref scheme) = scv::StyleSchemeManager::new().scheme("Adwaita-dark") {
                    buf.set_style_scheme(Some(scheme));
                }

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
