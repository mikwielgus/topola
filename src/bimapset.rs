// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
};

/// A bidirectional map between keys and sets of values.
///
/// - Each key can have multiple associated values (`BTreeSet<V>`).
/// - Each value maps to exactly one key (i.e., it's unique across keys).
#[derive(Clone, Debug, Default)]
pub struct BiBTreeMapSet<K, V> {
    key_to_values: BTreeMap<K, BTreeSet<V>>, // Forward mapping: key -> set of values.
    value_to_key: BTreeMap<V, K>,            // Reverse mapping: value -> key.
}

impl<K: Eq + Ord + Clone, V: Eq + Ord + Clone> BiBTreeMapSet<K, V> {
    /// Creates a new, empty `BiBTreeMapSet`.
    pub fn new() -> Self {
        Self {
            key_to_values: BTreeMap::new(),
            value_to_key: BTreeMap::new(),
        }
    }

    /// Inserts a (key, value) pair.
    ///
    /// If the value was previously associated with a different key,
    /// it is removed from the old key and reassigned to the new one.
    pub fn insert(&mut self, key: K, value: V) {
        // Insert the new value-to-key mapping, capturing the old key if it existed.
        if let Some(old_key) = self.value_to_key.insert(value.clone(), key.clone()) {
            // Remove value from the old key's set of values.
            if let Some(set) = self.key_to_values.get_mut(&old_key) {
                set.remove(&value);
                // Clean up the old key if its set becomes empty.
                if set.is_empty() {
                    self.key_to_values.remove(&old_key);
                }
            }
        }

        // Insert value into the new key's set of values.
        self.key_to_values
            .entry(key)
            .or_insert_with(BTreeSet::new)
            .insert(value);
    }

    /// Gets the set of values associated with the given key.
    pub fn get_values<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&BTreeSet<V>>
    where
        K: Borrow<Q>,
    {
        self.key_to_values.get(key)
    }

    /// Gets the key associated with the given value.
    pub fn get_key(&self, value: &V) -> Option<&K> {
        self.value_to_key.get(value)
    }

    /// Removes a key from the forward map and all its associated values from
    /// the reverse map.
    pub fn remove_by_key<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<BTreeSet<V>>
    where
        K: Borrow<Q>,
    {
        if let Some(values) = self.key_to_values.remove(key) {
            // Remove each value from the reverse map.
            for v in &values {
                self.value_to_key.remove(v);
            }
            Some(values)
        } else {
            None
        }
    }

    /// Removes a value from the reverse map and the key from the forward map if
    /// it no longer has any values.
    pub fn remove_by_value(&mut self, value: &V) -> Option<K> {
        if let Some(key) = self.value_to_key.remove(value) {
            // Remove the value from the key's value set.
            if let Some(set) = self.key_to_values.get_mut(&key) {
                set.remove(value);
                // Remove the key if it no longer has any values.
                if set.is_empty() {
                    self.key_to_values.remove(&key);
                }
            }
            Some(key)
        } else {
            None
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.key_to_values.keys()
    }
}
