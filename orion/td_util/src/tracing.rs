/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Utilities for working with the `tracing` crate.
//! Ensure all supertd projects have a consistent way of logging.

use std::io::{stderr, stdout, IsTerminal};

use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

/// Set up tracing so it prints to stderr, and can be used for output.
/// Most things should use `info` and `debug` level for showing messages.
pub fn init_tracing() {
    init_tracing_with_level(LevelFilter::INFO);
}

/// Set up tracing with a specific default log level.
pub fn init_tracing_with_level(default_level: LevelFilter) {
    let mut env_filter = EnvFilter::from_default_env();
    if std::env::var_os("RUST_LOG").is_none() {
        // Enable the specified log level by default
        env_filter = env_filter.add_directive(default_level.into());

        // Only add debug directives if we're not using WARN level
        if default_level != LevelFilter::WARN {
            // Debug log for target determinator packages
            let directives = [
                "btd=debug",
                "clients=debug",
                "ranker=debug",
                "rerun=debug",
                "scheduler=debug",
                "targets=debug",
                "verifiable=debug",
                "verifiable_matcher=debug",
                "verse=debug",
            ];
            for directive in directives {
                env_filter = env_filter
                    .add_directive(directive.parse().expect("bad hardcoded log directive"));
            }
        }
    }

    let layer = tracing_subscriber::fmt::layer()
        .with_line_number(false)
        .with_file(false)
        .with_writer(stderr)
        .with_ansi(stdout().is_terminal())
        .with_target(false)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(layer).init();
}
