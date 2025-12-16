/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Simple interface for logging to the `supertd_events` dataset.

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(not(target_os = "linux"))]
pub use non_linux::*;

#[cfg(not(target_os = "linux"))]
mod non_linux {
    pub fn init(_fb: fbinit::FacebookInit) {}

    #[macro_export]
    macro_rules! scuba_logger {
        ( event: $event:ident $(, $key:ident : $value:expr)* $(,)? ) => {};
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::env::var;
    use std::path::Path;
    use std::sync::OnceLock;

    use build_info::BuildInfo;
    use supertd_events_rust_logger::SupertdEventsLogEntry;
    use supertd_events_rust_logger::SupertdEventsLogger;

    static LOG_ENTRY: OnceLock<SupertdEventsLogEntry> = OnceLock::new();
    static FB_INIT: OnceLock<fbinit::FacebookInit> = OnceLock::new();

    /// Initialize the Scuba client for the `supertd_events` dataset.
    ///
    /// Returns a guard that flushes the Scuba client when dropped.
    ///
    /// Expects `tracing` to be initialized.
    pub fn init(fb: fbinit::FacebookInit) {
        if FB_INIT.set(fb).is_err() {
            tracing::error!("supertd_events client initialized twice");
        }
        let mut log_entry = SupertdEventsLogEntry::default();
        add_common_server_data(&mut log_entry);
        add_sandcastle_columns(&mut log_entry);
        if LOG_ENTRY.set(log_entry).is_err() {
            tracing::error!("supertd_events Scuba client initialized twice");
        }
    }

    /// Log a sample to the `supertd_events` dataset.
    ///
    /// The `event` column should be a distinct string for each source location
    /// logging an event.
    ///
    /// The `data` column contains JSON-encoded data specific to that event (so that
    /// we do not inflate the number of columns in the Scuba table with properties
    /// populated by only one event). Use this data in derived columns or queries
    /// using `JSON_EXTRACT`.
    ///
    /// If [`init`] has not been invoked, the sample will not be logged.
    ///
    /// # Examples
    ///
    /// ```
    /// # let f = || (10, 2);
    /// let t = std::time::Instant::now();
    /// let (foos_run, bars_launched) = f();
    /// td_util::scuba!(
    ///     event: BTD_SUCCESS,
    ///     duration: t.elapsed(),
    ///     data: json!({
    ///         "arbitrary": ["JSON", "object"],
    ///         "foos_run": foos_run,
    ///         "bars_launched": bars_launched,
    ///     })
    /// );
    /// ```
    #[macro_export]
    macro_rules! scuba_logger {
    ( event: $event:ident $(, $key:ident : $value:expr)* $(,)? ) => {
        let mut builder = $crate::supertd_events_logger::log_entry();
        builder.set_event(format!("{:?}", &$crate::supertd_events::Event::$event));
        $($crate::scuba_logger! { @SET_FIELD(builder, $key, $value) })*
        $crate::supertd_events_logger::log(&builder);
    };
    ( $($key:ident : $value:expr),* $(,)? ) => {
        compile_error!("`event` must be the first field in the `scuba!` macro");
    };
    ( @SET_FIELD ( $builder:ident, event, $value:expr ) ) => {
        compile_error!("duplicate `event` field in `scuba!` macro");
    };
    ( @SET_FIELD ( $builder:ident, data, $value:expr ) ) => {{
        use $crate::supertd_events::serde_json::json;
        match $crate::supertd_events::serde_json::to_string(&$value) {
            Ok(json) => {
                $builder.set_data(json);
            }
            Err(e) => {
                $crate::supertd_events::tracing::error!(
                    "Failed to serialize `data` column in `scuba!` macro: {:?}", e);
            }
        }
    }};
    ( @SET_FIELD ( $builder:ident, duration, $value:expr ) ) => {
        $builder.set_duration_ms(::std::time::Duration::as_millis(&$value) as i64);
    };
    ( @SET_FIELD ( $builder:ident, duration_ms, $value:expr ) ) => {
        compile_error!("unrecognized column name in `scuba!` macro: duration_ms (use `duration` instead)");
    };
    ( @SET_FIELD ( $builder:ident, $key:ident, $value:expr ) ) => {
        compile_error!(concat!("unrecognized column name in `scuba!` macro: ", stringify!($key)));
    };
}

    /// Get the log_entry for the `supertd_events` dataset.
    ///
    /// Please use the [`scuba!`] macro instead of this function, since it provides
    /// additional type safety (e.g., prevents typos in column names). This function
    /// is exposed only for internal use by the macro.
    #[doc(hidden)]
    pub fn log_entry() -> SupertdEventsLogEntry {
        LOG_ENTRY.get().cloned().unwrap_or_default()
    }

    #[doc(hidden)]
    pub fn log(log_entry: &SupertdEventsLogEntry) {
        if let Some(&fb) = FB_INIT.get() {
            if let Err(e) = SupertdEventsLogger::from_entry(fb, log_entry).log() {
                tracing::error!("Failed to flush supertd_events Scuba: {:?}", e);
            }
        }
    }

    fn add_common_server_data(log_entry: &mut SupertdEventsLogEntry) {
        if let Ok(who) = fbwhoami::FbWhoAmI::get() {
            if let Some(hostname) = who.name.as_deref() {
                log_entry.set_server_hostname(hostname.to_owned());
            }
            if let Some(region) = who.region.as_deref() {
                log_entry.set_region(region.to_owned());
            }
            if let Some(dc) = who.datacenter.as_deref() {
                log_entry.set_datacenter(dc.to_owned());
            }
            if let Some(dc_prefix) = who.region_datacenter_prefix.as_deref() {
                log_entry.set_region_datacenter_prefix(dc_prefix.to_owned());
            }
        }

        if let Ok(smc_tier) = var("SMC_TIERS") {
            log_entry.set_server_tier(smc_tier);
        }

        if let Ok(tw_task_id) = var("TW_TASK_ID") {
            log_entry.set_tw_task_id(tw_task_id);
        }

        if let Ok(tw_canary_id) = var("TW_CANARY_ID") {
            log_entry.set_tw_canary_id(tw_canary_id);
        }

        if let (Ok(tw_cluster), Ok(tw_user), Ok(tw_name)) = (
            var("TW_JOB_CLUSTER"),
            var("TW_JOB_USER"),
            var("TW_JOB_NAME"),
        ) {
            log_entry.set_tw_handle(format!("{}/{}/{}", tw_cluster, tw_user, tw_name));
        };

        if let (Ok(tw_cluster), Ok(tw_user), Ok(tw_name), Ok(tw_task_id)) = (
            var("TW_JOB_CLUSTER"),
            var("TW_JOB_USER"),
            var("TW_JOB_NAME"),
            var("TW_TASK_ID"),
        ) {
            log_entry.set_tw_task_handle(format!(
                "{}/{}/{}/{}",
                tw_cluster, tw_user, tw_name, tw_task_id
            ));
        };

        #[cfg(target_os = "linux")]
        {
            log_entry.set_build_revision(BuildInfo::get_revision().to_owned());
            log_entry.set_build_rule(BuildInfo::get_rule().to_owned());
        }

        #[cfg(target_os = "linux")]
        log_entry.set_operating_system("linux".to_owned());

        #[cfg(target_os = "macos")]
        log_entry.set_operating_system("macos".to_owned());

        #[cfg(target_os = "windows")]
        log_entry.set_operating_system("windows".to_owned());
    }

    fn apply_verifiable(var: &str, variables_path: &Path, f: impl FnOnce(String)) {
        if let Ok(value) = std::fs::read_to_string(variables_path.join(var)) {
            f(value);
        } else if let Ok(value) = std::env::var(var) {
            f(value);
        }
    }

    fn add_sandcastle_columns(log_entry: &mut SupertdEventsLogEntry) {
        let Some(nexus_path) = std::env::var_os("SANDCASTLE_NEXUS") else {
            return;
        };
        let nexus_path = std::path::Path::new(&nexus_path);
        if !nexus_path.exists() {
            return;
        }
        let variables_path = nexus_path.join("variables");
        apply_verifiable("SANDCASTLE_ALIAS_NAME", &variables_path, |value| {
            log_entry.set_sandcastle_alias_name(value);
        });
        apply_verifiable("SANDCASTLE_ALIAS", &variables_path, |value| {
            log_entry.set_sandcastle_alias(value);
        });
        apply_verifiable("SANDCASTLE_COMMAND_NAME", &variables_path, |value| {
            log_entry.set_sandcastle_command_name(value);
        });
        apply_verifiable("SANDCASTLE_INSTANCE_ID", &variables_path, |value| {
            log_entry.set_sandcastle_instance_id(value);
        });
        apply_verifiable("SANDCASTLE_IS_DRY_RUN", &variables_path, |value| {
            log_entry.set_sandcastle_is_dry_run(value);
        });
        apply_verifiable("SANDCASTLE_JOB_OWNER", &variables_path, |value| {
            log_entry.set_sandcastle_job_owner(value);
        });
        apply_verifiable("SANDCASTLE_NONCE", &variables_path, |value| {
            log_entry.set_sandcastle_nonce(value);
        });
        apply_verifiable("SANDCASTLE_PHABRICATOR_DIFF_ID", &variables_path, |value| {
            log_entry.set_sandcastle_phabricator_diff_id(value);
        });
        apply_verifiable("SANDCASTLE_SCHEDULE_TYPE", &variables_path, |value| {
            log_entry.set_sandcastle_schedule_type(value);
        });
        apply_verifiable("SANDCASTLE_TYPE", &variables_path, |value| {
            log_entry.set_sandcastle_type(value);
        });
        apply_verifiable("SANDCASTLE_URL", &variables_path, |value| {
            log_entry.set_sandcastle_url(value);
        });
        apply_verifiable("SKYCASTLE_ACTION_ID", &variables_path, |value| {
            log_entry.set_skycastle_action_id(value);
        });
        apply_verifiable("SKYCASTLE_JOB_ID", &variables_path, |value| {
            log_entry.set_skycastle_job_id(value);
        });
        apply_verifiable("SKYCASTLE_WORKFLOW_RUN_ID", &variables_path, |value| {
            log_entry.set_skycastle_workflow_run_id(value);
        });
    }
}
