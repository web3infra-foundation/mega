// Copyright (C) 2023 Ant Group. All rights reserved.
//  2024 From [fuse_backend_rs](https://github.com/cloud-hypervisor/fuse-backend-rs) 
// SPDX-License-Identifier: Apache-2.0

use self::super::CachePolicy;
use std::fmt;
use std::time::Duration;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub mountpoint: String,
    pub work: String,
    pub do_import: bool,
    // Filesystem options.
    pub writeback: bool,
    pub no_open: bool,
    pub no_opendir: bool,
    pub killpriv_v2: bool,
    pub no_readdir: bool,
    pub perfile_dax: bool,
    pub cache_policy: CachePolicy,
    pub attr_timeout: Duration,
    pub entry_timeout: Duration,
}

impl Clone for CachePolicy {
    fn clone(&self) -> Self {
        match *self {
            CachePolicy::Never => CachePolicy::Never,
            CachePolicy::Always => CachePolicy::Always,
            CachePolicy::Auto => CachePolicy::Auto,
        }
    }
}

impl fmt::Debug for CachePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let policy = match *self {
            CachePolicy::Never => "Never",
            CachePolicy::Always => "Always",
            CachePolicy::Auto => "Auto",
        };

        write!(f, "CachePolicy: {}", policy)
    }
}
