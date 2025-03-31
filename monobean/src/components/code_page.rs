use std::path::Path;

use async_channel::Sender;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use scv::{prelude::*, Buffer};

use crate::application::Action;

mod imp {
    use tokio::sync::OnceCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/code_page.ui")]
    pub struct CodePage {
        // #[template_child]
        // pub searchbar: TemplateChild<gtk::SearchBar>,
        // #[template_child]
        // pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub listview: TemplateChild<gtk::ListView>,
        #[template_child]
        pub code_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub empty_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub source_view: TemplateChild<scv::View>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CodePage {
        const NAME: &'static str = "CodePage";
        type Type = super::CodePage;
        type ParentType = gtk::Box;

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
    impl BoxImpl for CodePage {}
}

glib::wrapper! {
    pub struct CodePage(ObjectSubclass<imp::CodePage>)
        @extends gtk::Widget, gtk::Box,
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

        let buf = Buffer::new(None);
        buf.set_highlight_syntax(true);
        if let Some(ref language) = scv::LanguageManager::new().language("rust") {
            buf.set_language(Some(language));
        }
        // if let Some(ref scheme) = scv::StyleSchemeManager::new().scheme("solarized-dark") {
        //     buf.set_style_scheme(Some(scheme));
        // }

        // FIXME: be care with os path
        let pb = std::path::PathBuf::from("E:/Projects/mega/monobean/src/components/code_page.rs");
        let file = adw::gio::File::for_path(&pb);
        let file = scv::File::builder().location(&file).build();
        let loader = scv::FileLoader::new(&buf, &file);
        loader.load_async_with_callback(
            glib::Priority::default(),
            adw::gio::Cancellable::NONE,
            move |current_num_bytes, total_num_bytes| {
                println!(
                    "loading: {:?}",
                    (current_num_bytes as f32 / total_num_bytes as f32) * 100f32
                );
            },
            |res| {
                println!("loaded: {:?}", res);
            },
        );

        self.imp().source_view.set_buffer(Some(&buf));
        self.imp().code_stack.set_visible_child_name("source_view");
    }
}
