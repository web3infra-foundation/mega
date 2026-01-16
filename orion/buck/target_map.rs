/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashMap;

use td_util::no_hash::BuildNoHash;
use tracing::warn;

use crate::{
    package_resolver::PackageResolver,
    types::{Package, TargetLabel, TargetPattern},
};

pub struct TargetMap<T> {
    literal: HashMap<TargetLabel, Vec<T>, BuildNoHash>,
    non_recursive_pattern: HashMap<Package, Vec<T>, BuildNoHash>,
    recursive_pattern: PackageResolver<Vec<T>>,
}

impl<T> Default for TargetMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TargetMap<T> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// The capacity is used for `insert` capacity, rather than `insert_pattern` capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            literal: HashMap::with_capacity_and_hasher(capacity, BuildNoHash::default()),
            non_recursive_pattern: HashMap::with_hasher(BuildNoHash::default()),
            recursive_pattern: PackageResolver::new(),
        }
    }

    pub fn insert(&mut self, key: &TargetLabel, value: T) {
        self.literal.entry(key.clone()).or_default().push(value)
    }

    pub fn insert_pattern(&mut self, key: &TargetPattern, value: T) {
        if let Some(label) = key.as_target_label() {
            self.insert(&label, value);
        } else if let Some(prefix) = key.as_package_pattern() {
            self.non_recursive_pattern
                .entry(prefix)
                .or_default()
                .push(value);
        } else if let Some(prefix) = key.as_recursive_pattern() {
            self.recursive_pattern.update(&prefix, move |old| {
                let mut res = old.unwrap_or_default();
                res.push(value);
                res
            });
        } else {
            warn!("Ignored invalid target pattern, `{}`", key)
        }
    }

    pub fn get<'a, 'b>(&'a self, key: &'b TargetLabel) -> impl Iterator<Item = &'a T> + 'b
    where
        'a: 'b,
    {
        let package = key.package();
        let literals = self.literal.get(key).into_iter().flatten();
        let non_recursive_patterns = self
            .non_recursive_pattern
            .get(&package)
            .into_iter()
            .flatten();
        let recursive_patterns = self.recursive_pattern.get(&package).into_iter().flatten();
        literals
            .chain(non_recursive_patterns)
            .chain(recursive_patterns)
    }

    pub fn is_terminal_node(&self, key: &TargetLabel) -> bool {
        // If the corresponding entry for the given TargetLabel doesn't
        // contain any values, where each value is an edge between two nodes,
        // then we can conclude that we have a terminal node.
        // In a normal dependency graph, this would be the leaf targets.
        // In a reverse dependency graph, this would be the root targets.
        match self.literal.get(key) {
            Some(values) => values.is_empty(),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_map() {
        let mut t: TargetMap<i32> = TargetMap::new();
        t.insert(&TargetLabel::new("foo//bar:baz"), 1);
        t.insert_pattern(&TargetPattern::new("foo//bar:baz"), 2);
        t.insert(&TargetLabel::new("foo//bar:quz"), 100);
        t.insert_pattern(&TargetPattern::new("foo//bar:"), 3);
        t.insert_pattern(&TargetPattern::new("foo//..."), 4);
        t.insert_pattern(&TargetPattern::new("foo//bar/..."), 5);
        assert_eq!(
            t.get(&TargetLabel::new("foo//bar:baz"))
                .copied()
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            t.get(&TargetLabel::new("foo//moo:boo"))
                .copied()
                .collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(
            t.get(&TargetLabel::new("none//moo:boo"))
                .copied()
                .collect::<Vec<_>>(),
            Vec::<i32>::new()
        );
    }
}
