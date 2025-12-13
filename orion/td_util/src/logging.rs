/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use tracing::info;

static START_TIME: OnceLock<Instant> = OnceLock::new();

pub fn init_logger_start_time() {
    START_TIME
        .set(Instant::now())
        .expect("START_TIME already initialized");
}

pub fn start_time() -> Instant {
    START_TIME.get_or_init(Instant::now).clone()
}

pub fn elapsed() -> Duration {
    start_time().elapsed()
}

pub fn step(name: &str) {
    info!("Starting {} at {:.3}s", name, elapsed().as_secs_f64());
}
