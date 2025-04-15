use adw::gio::Settings;
use adw::glib;
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use adw::subclass::prelude::{AdwWindowImpl, PreferencesWindowImpl};
use gtk::subclass::prelude::*;
use gtk::{Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager};
use gtk::{CompositeTemplate, Entry, SpinButton, Switch};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/Web3Infrastructure/Monobean/gtk/preferences.ui")]
    pub struct MonobeanPreferences {
        pub settings: OnceCell<Settings>,

        // Base Settings
        #[template_child]
        pub base_dir_entry: TemplateChild<Entry>,

        // Logging Settings
        #[template_child]
        pub log_path_entry: TemplateChild<Entry>,
        #[template_child]
        pub log_level: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub print_std_switch: TemplateChild<Switch>,

        // DB Settings
        #[template_child]
        pub db_type: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub db_path_entry: TemplateChild<Entry>,
        #[template_child]
        pub db_url_entry: TemplateChild<Entry>,
        #[template_child]
        pub max_connection_spin: TemplateChild<SpinButton>,
        #[template_child]
        pub min_connection_spin: TemplateChild<SpinButton>,
        #[template_child]
        pub sqlx_logging_switch: TemplateChild<Switch>,

        // Auth Settings
        #[template_child]
        pub http_auth_switch: TemplateChild<Switch>,
        #[template_child]
        pub test_user_switch: TemplateChild<Switch>,
        #[template_child]
        pub test_user_name_entry: TemplateChild<Entry>,
        #[template_child]
        pub test_user_token_entry: TemplateChild<Entry>,

        // Storage Settings
        #[template_child]
        pub obs_access_key_entry: TemplateChild<Entry>,
        #[template_child]
        pub obs_secret_key_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub obs_region_entry: TemplateChild<Entry>,
        #[template_child]
        pub obs_endpoint_entry: TemplateChild<Entry>,

        // Monorepo Settings
        #[template_child]
        pub import_dir_entry: TemplateChild<Entry>,
        #[template_child]
        pub admin_entry: TemplateChild<Entry>,
        #[template_child]
        pub root_dirs_entry: TemplateChild<Entry>,

        // Pack Settings
        #[template_child]
        pub pack_decode_mem_size_entry: TemplateChild<Entry>,
        #[template_child]
        pub pack_decode_disk_size_entry: TemplateChild<Entry>,
        #[template_child]
        pub pack_decode_cache_path_entry: TemplateChild<Entry>,
        #[template_child]
        pub clean_cache_switch: TemplateChild<Switch>,
        #[template_child]
        pub channel_message_size_spin: TemplateChild<SpinButton>,

        // LFS Settings
        #[template_child]
        pub lfs_url_entry: TemplateChild<Entry>,

        // OAuth Settings
        #[template_child]
        pub github_client_id_entry: TemplateChild<Entry>,
        #[template_child]
        pub github_client_secret_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub ui_domain_entry: TemplateChild<Entry>,
        #[template_child]
        pub cookie_domain_entry: TemplateChild<Entry>,

        // P2P Settings
        #[template_child]
        pub bootstrap_node: TemplateChild<Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MonobeanPreferences {
        const NAME: &'static str = "MonobeanPreferences";
        type Type = super::MonobeanPreferences;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MonobeanPreferences {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.setup_settings();
            obj.bind_settings();
        }
    }
    impl WidgetImpl for MonobeanPreferences {}
    impl WindowImpl for MonobeanPreferences {}
    impl AdwWindowImpl for MonobeanPreferences {}
    impl PreferencesWindowImpl for MonobeanPreferences {}
}

glib::wrapper! {
    pub struct MonobeanPreferences(ObjectSubclass<imp::MonobeanPreferences>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow,
        @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl MonobeanPreferences {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    fn bind_settings(&self) {
        let settings = self.settings();
        let imp = self.imp();

        // Base Settings
        settings
            .bind("base-dir", &imp.base_dir_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // Logging Settings
        settings
            .bind("log-path", &imp.log_path_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        settings
            .bind("log-level", &imp.log_level.get(), "selected")
            .mapping(|variant, _| {
                let level = variant.get::<String>().unwrap();
                let index = match level.as_str() {
                    "debug" => 0,
                    "info" => 1,
                    "warning" => 2,
                    "error" => 3,
                    _ => 1, // 默认 info
                };
                Some(index.to_value())
            })
            .set_mapping(|variant, _| {
                let index = variant.get::<u32>().unwrap();
                let level = match index {
                    0 => "debug",
                    1 => "info",
                    2 => "warning",
                    3 => "error",
                    _ => "info",
                };
                Some(level.to_variant())
            })
            .build();
        settings
            .bind("print-std", &imp.print_std_switch.get(), "active")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // DB Settings
        settings
            .bind("db-type", &imp.db_type.get(), "selected")
            .mapping(|variant, _| {
                let db_type = variant.get::<String>().unwrap();
                let index = match db_type.as_str() {
                    "sqlite" => 0,
                    "postgres" => 1,
                    _ => 0,
                };
                Some(index.to_value())
            })
            .set_mapping(|variant, _| {
                let index = variant.get::<u32>().unwrap();
                let db_type = match index {
                    0 => "sqlite",
                    1 => "postgres",
                    _ => "sqlite",
                };
                Some(db_type.to_variant())
            })
            .build();
        settings
            .bind("db-path", &imp.db_path_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("db-url", &imp.db_url_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("max-connections", &imp.max_connection_spin.get(), "value")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("min-connections", &imp.min_connection_spin.get(), "value")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("sqlx-logging", &imp.sqlx_logging_switch.get(), "active")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // Auth Settings
        settings
            .bind("http-auth", &imp.http_auth_switch.get(), "active")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("test-user", &imp.test_user_switch.get(), "active")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("test-user-name", &imp.test_user_name_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("test-user-token", &imp.test_user_token_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // Storage Settings
        settings
            .bind("obs-access-key", &imp.obs_access_key_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("obs-secret-key", &imp.obs_secret_key_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("obs-region", &imp.obs_region_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("obs-endpoint", &imp.obs_endpoint_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // Monorepo Settings
        settings
            .bind("import-dir", &imp.import_dir_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("admin", &imp.admin_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("root-dirs", &imp.root_dirs_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // Pack Settings
        settings
            .bind(
                "pack-decode-mem-size",
                &imp.pack_decode_mem_size_entry.get(),
                "text",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind(
                "pack-decode-disk-size",
                &imp.pack_decode_disk_size_entry.get(),
                "text",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind(
                "pack-decode-cache-path",
                &imp.pack_decode_cache_path_entry.get(),
                "text",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("clean-cache", &imp.clean_cache_switch.get(), "active")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind(
                "channel-message-size",
                &imp.channel_message_size_spin.get(),
                "value",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // LFS Settings
        settings
            .bind("lfs-url", &imp.lfs_url_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // OAuth Settings
        settings
            .bind(
                "github-client-id",
                &imp.github_client_id_entry.get(),
                "text",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind(
                "github-client-secret",
                &imp.github_client_secret_entry.get(),
                "text",
            )
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("ui-domain", &imp.ui_domain_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
        settings
            .bind("cookie-domain", &imp.cookie_domain_entry.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();

        // P2P Settings
        settings
            .bind("bootstrap-node", &imp.bootstrap_node.get(), "text")
            .flags(adw::gio::SettingsBindFlags::DEFAULT)
            .build();
    }
}

impl Default for MonobeanPreferences {
    fn default() -> Self {
        Self::new()
    }
}
