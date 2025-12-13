/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

// Moved all `cfg(fbcode_build)`

pub fn check_boolean_knob(_name: &str) -> bool {
    false
}

pub fn check_boolean_knob_with_switch(
    _name: &str,
    _switch_val: Option<&str>,
    default: bool,
) -> bool {
    default
}

pub fn check_boolean_knob_with_switch_and_consistent_pass_rate(
    _name: &str,
    _hash_val: Option<&str>,
    _switch_val: Option<&str>,
    default: bool,
) -> bool {
    default
}

pub fn check_integer_knob(_name: &str, default_value: i64) -> i64 {
    default_value
}

pub fn check_integer_knob_with_switch(
    _name: &str,
    _switch_val: Option<&str>,
    default_value: i64,
) -> i64 {
    default_value
}