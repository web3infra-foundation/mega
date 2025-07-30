use crate::application::Action;
use crate::core::mega_core::MegaCommands;
use crate::CONTEXT;
use adw::glib::{clone, timeout_future, GString, Regex, RegexCompileFlags, RegexMatchFlags};
use adw::prelude::*;
use async_channel::Sender;
use gtk::glib::random_int_range;
use gtk::prelude::{ButtonExt, EditableExt, WidgetExt};
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use std::time::Duration;
use tokio::sync::oneshot;

mod imp {
    use super::*;
    use crate::application::Action;

    use async_channel::Sender;

    use std::cell::OnceCell;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/hello_page.ui")]
    pub struct HelloPage {
        #[template_child]
        pub name_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub email_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub continue_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub logo: TemplateChild<gtk::Image>,
        #[template_child]
        pub pgp_row: TemplateChild<adw::PreferencesRow>,
        #[template_child]
        pub pgp_button: TemplateChild<gtk::Button>,
        // #[template_child]
        // pub pgp_spin: TemplateChild<adw::Spinner>,
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
    impl HelloPage {}
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
        self.setup_logo();
        self.setup_action();
    }

    fn setup_logo(&self) {
        let logo = self.imp().logo.clone();
        let id = random_int_range(1, 6);
        logo.set_icon_name(Some(format!("walrus-{id}").as_str()));

        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(clone!(
            #[weak]
            logo,
            move |_, _, _, _| {
                let id = random_int_range(1, 6);
                logo.set_icon_name(Some(format!("walrus-{id}").as_str()));
            }
        ));
        logo.add_controller(gesture);
    }

    fn setup_action(&self) {
        let sender = self.imp().sender.get().unwrap().clone();
        let continue_button = self.imp().continue_button.clone();
        let email_entry = self.imp().email_entry.clone();
        let name_entry = self.imp().name_entry.clone();
        let pgp_button = self.imp().pgp_button.clone();

        email_entry.connect_changed(clone!(
            #[weak(rename_to=page)]
            self,
            #[weak]
            continue_button,
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            #[weak]
            pgp_button,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();

                continue_button.set_sensitive(page.should_continue(
                    &name,
                    &email,
                    pgp_button.is_sensitive(),
                ));
                pgp_button.set_sensitive(true);
            }
        ));

        name_entry.connect_changed(clone!(
            #[weak(rename_to=page)]
            self,
            #[weak]
            continue_button,
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            #[weak]
            pgp_button,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();

                continue_button.set_sensitive(page.should_continue(
                    &name,
                    &email,
                    pgp_button.is_sensitive(),
                ));
                pgp_button.set_sensitive(true);
            }
        ));

        pgp_button.connect_clicked(clone!(
            #[weak(rename_to=page)]
            self,
            move |_| page.setup_pgp()
        ));

        continue_button.connect_clicked(clone!(
            #[weak]
            email_entry,
            #[weak]
            name_entry,
            #[strong]
            sender,
            move |_| {
                let email = email_entry.text();
                let name = name_entry.text();
                sender
                    .send_blocking(Action::UpdateGitConfig(name.to_string(), email.to_string()))
                    .unwrap();
                sender.send_blocking(Action::ShowMainPage).unwrap();
            }
        ));
    }

    pub fn setup_pgp(&self) {
        let imp = self.imp();

        let sender = imp.sender.get().unwrap().clone();
        let continue_button = imp.continue_button.clone();
        let email_entry = imp.email_entry.clone();
        let name_entry = imp.name_entry.clone();
        let pgp_row = imp.pgp_row.clone();
        let pgp_button = imp.pgp_button.clone();

        let email = email_entry.text();
        let name = name_entry.text();

        if name.is_empty() || email.is_empty() {
            tracing::debug!("Empty email entry----------");
            return;
        }

        tracing::debug!("--------name: {} and email {}-----", name, email);

        let (tx, rx) = oneshot::channel();
        let user_name = name.clone();
        let user_email = email.clone();

        let pgp_command = Action::MegaCore(MegaCommands::LoadOrInitPgp {
            chan: tx,
            user_name: user_name.parse().unwrap(),
            user_email: user_email.parse().unwrap(),
            passwd: None,
        });

        pgp_button.set_sensitive(false);

        let spinner = pgp_button.prev_sibling().unwrap();

        let should_continue = self.should_continue(&name, &email, pgp_button.is_sensitive());

        CONTEXT.spawn_local(async move {
            spinner.set_visible(true);
            sender.send(pgp_command).await.unwrap();

            if rx.await.is_err() {
                sender
                    .send(Action::AddToast("Failed to init pgp key".to_string()))
                    .await
                    .unwrap();
                pgp_button.set_sensitive(true);
            } else {
                pgp_row.set_title("PGP key already generated");
                continue_button.set_sensitive(should_continue);
                sender
                    .send(Action::AddToast("PGP key initialized".to_string()))
                    .await
                    .unwrap();
            }
            spinner.set_visible(false);
        });
    }

    fn should_continue(&self, name: &GString, email: &GString, btn_sensitive: bool) -> bool {
        let re = Regex::new(
            r"^\w+(-+.\w+)*@\w+(-.\w+)*.\w+(-.\w+)*$",
            RegexCompileFlags::DEFAULT,
            RegexMatchFlags::DEFAULT,
        )
        .unwrap()
        .unwrap();

        // Glib Regex asserts the input string doesn't have a suffix '\0' or it will panic.
        let email = email.trim();
        let email = GString::from(email);

        let result = !name.trim().is_empty()
            && !email.is_empty()
            && !btn_sensitive
            && re
                .match_full(email.as_gstr(), 0, RegexMatchFlags::DEFAULT)
                .is_ok();
        result
    }

    pub fn fill_entries(&self, name: Option<String>, email: Option<String>, pgp_generated: bool) {
        if let Some(name) = name {
            self.imp().name_entry.set_text(&name);
        }
        if let Some(email) = email {
            self.imp().email_entry.set_text(&email);
        }
        if pgp_generated {
            self.imp().pgp_button.set_sensitive(false);
            self.imp().pgp_row.set_title("PGP key already generated");
        }
        self.imp()
            .continue_button
            .set_sensitive(self.should_continue(
                &self.imp().name_entry.text(),
                &self.imp().email_entry.text(),
                self.imp().pgp_button.is_sensitive(),
            ));

        tracing::debug!(
            "--------name: {} and email {}-----",
            self.imp().name_entry.text(),
            self.imp().email_entry.text()
        );

        glib::MainContext::default().spawn_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                timeout_future(Duration::from_millis(1500)).await;
                this.setup_pgp();
            }
        ));
    }
}

impl Default for HelloPage {
    fn default() -> Self {
        Self::new()
    }
}
