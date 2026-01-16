/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! A simplified hashing implementation that only supports hashers that write a single u64.
//! Avoids swizzling the bits. Useful for reducing overhead in cases, particularly when
//! using `InternString`.

use std::hash::{BuildHasherDefault, Hasher};

pub type BuildNoHash = BuildHasherDefault<NoHash>;

#[derive(Default, Debug)]
pub struct NoHash(u64);

impl Hasher for NoHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _bytes: &[u8]) {
        panic!("Invalid use of NoHash")
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(u64::from(n));
    }

    fn write_u16(&mut self, n: u16) {
        self.write_u64(u64::from(n));
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        // Check we haven't called it twice
        debug_assert_eq!(self.0, 0);
        self.0 = n;
    }

    fn write_u128(&mut self, n: u128) {
        // Fold into u64 deterministically.
        self.write_u64((n as u64) ^ ((n >> 64) as u64));
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    fn write_i8(&mut self, n: i8) {
        self.write_u64(n as u64);
    }

    fn write_i16(&mut self, n: i16) {
        self.write_u64(n as u64);
    }

    fn write_i32(&mut self, n: i32) {
        self.write_u64(n as u64);
    }

    fn write_i64(&mut self, n: i64) {
        self.write_u64(n as u64);
    }

    fn write_i128(&mut self, n: i128) {
        self.write_u128(n as u128);
    }

    fn write_isize(&mut self, n: isize) {
        self.write_u64(n as u64);
    }
}
