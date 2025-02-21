use crate::application::Action;
use adw::glib::{clone, GString};
use async_channel::Sender;
use gtk::prelude::{ButtonExt, EditableExt, WidgetExt};
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;
    use crate::application::Action;

    use async_channel::Sender;

    use std::cell::OnceCell;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/hello_page.ui")]
    pub struct HelloPage {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub primary_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub name_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub email_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub continue_button: TemplateChild<gtk::Button>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HelloPage {
        const NAME: &'static str = "HelloPage";
        type Type = super::HelloPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HelloPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for HelloPage {}
    impl BoxImpl for HelloPage {}
    
    #[gtk::template_callbacks]
    impl HelloPage {
        
    }
}

glib::wrapper! {
    pub struct HelloPage(ObjectSubclass<imp::HelloPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl HelloPage {
    pub fn new() -> Self {
        glib::object::Object::new()
    }

    pub fn setup_hello_page(&self, sender: Sender<Action>) {
        self.imp()
            .sender
            .set(sender)
            .expect("Hello Page sender can only be set once");
        self.setup_action();
    }

    fn setup_action(&self) {
        let sender = self.imp().sender.get().unwrap().clone();
        let continue_button = self.imp().continue_button.clone();
        let email_entry = self.imp().email_entry.clone();
        let name_entry = self.imp().name_entry.clone();

        let should_continue = move |name: GString, email: GString| -> bool {
            // FIXME: There's a bug in glib regex, 
            // we have to find a temporary solution for this.
            
            // let re = Regex::new(
            //     r"^\w+(-+.\w+)*@\w+(-.\w+)*.\w+(-.\w+)*$",
            //     RegexCompileFlags::DEFAULT,
            //     RegexMatchFlags::DEFAULT,
            // )
            // .unwrap()
            // .unwrap();
            // 
            // re.match_full(email.as_ref(),0 , RegexMatchFlags::DEFAULT)
            //     .is_ok()
            //     && !name.is_empty()
            
            !name.is_empty() && !email.is_empty()
        };

        email_entry.connect_changed(clone!(
            #[weak]
            continue_button,
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();

                continue_button.set_sensitive(should_continue(name, email));
            }
        ));

        name_entry.connect_changed(clone!(
            #[weak]
            continue_button,
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();

                continue_button.set_sensitive(should_continue(name, email));
            }
        ));
        
        continue_button.connect_clicked(clone!(
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();
                sender.send_blocking(Action::UpdateGitConfig(name.to_string(), email.to_string())).unwrap();
                sender.send_blocking(Action::ShowMainPage).unwrap();
            }
        ));
    }

    pub fn fill_entries(&self, name: Option<String>, email: Option<String>) {
        if let Some(name) = name {
            self.imp().name_entry.set_text(&name);
        }
        if let Some(email) = email {
            self.imp().email_entry.set_text(&email);
        }
    }
}

impl Default for HelloPage {
    fn default() -> Self {
        Self::new()
    }
}
