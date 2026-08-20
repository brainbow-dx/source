//! Disclaimer: This is AI slop. I didn't want to write the ordered map by
//! hand, but this should be replaced with a human-written impl.
//!
//! Note: if you see this in the public repo, pls open a PR and berate
//! the repo maintainer (or whoever).

use allocator_api2::alloc::Allocator;
use allocator_api2::alloc::Global;
use allocator_api2::vec::Vec;

use core::borrow::Borrow;
use core::hash::BuildHasher;
use core::hash::Hash;

use hashbrown::DefaultHashBuilder;
use hashbrown::HashMap;

/// A map-like data structure that preserves the original insertion order of its keys.
///
/// It is implemented using a `Vec` to store the ordered keys and a `HashMap`
/// for fast O(1) lookups.
#[derive(Debug)]
pub struct OrderedMap<K, V, S = DefaultHashBuilder, A: Allocator = Global> {
    inner: HashMap<K, V, S, A>,
    index: Vec<K, A>,
}

impl<K, V> OrderedMap<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty `OrderedMap` with the global allocator.
    pub fn new() -> Self {
        Self {
            index: Vec::new(),
            inner: HashMap::new(),
        }
    }
}

impl<K, V, A> OrderedMap<K, V, DefaultHashBuilder, A>
where
    K: Eq + Hash,
    A: Allocator + Copy,
{
    /// Creates a new, empty `OrderedMap` with the given allocator.
    pub fn new_in(allocator: A) -> Self {
        Self {
            index: Vec::new_in(allocator),
            inner: HashMap::new_in(allocator),
        }
    }
}

// Default implementation
impl<K, V> Default for OrderedMap<K, V>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, S, A> OrderedMap<K, V, S, A>
where
    K: Eq + Hash + Clone,
    A: Allocator,
    S: BuildHasher,
{
    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned. If the map
    /// did have this key present, the value is updated, and the old value is
    /// returned. The key is not updated, and insertion order is not changed.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if !self.inner.contains_key(&key) {
            self.index.push(key.clone());
        }
        self.inner.insert(key, value)
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get(key)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// This is an O(n) operation because it requires finding and removing the
    /// key from the internal `Vec`.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let value = self.inner.remove(key);
        if value.is_some() {
            // Find the key in the `keys` Vec and remove it.
            if let Some(index) = self.index.iter().position(|k| k.borrow() == key) {
                self.index.remove(index);
            }
        }
        value
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns an iterator visiting all key-value pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.index.iter().map(move |key| (key, &self.inner[key]))
    }

    /// Returns an iterator visiting all keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.index.iter()
    }

    /// Returns an iterator visiting all values in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.index.iter().map(move |key| &self.inner[key])
    }
}
