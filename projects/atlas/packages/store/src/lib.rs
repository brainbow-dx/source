#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::sync::Arc;

use parking_lot::RwLock;

// --- The Core Trait ---
// This trait defines the public API for your concurrent map.
// It uses closures to provide a safe, non-blocking way to access and mutate
// the inner data without exposing the internal locks.

pub trait Store<K, V> {
    fn get(&self, key: &K) -> Option<Arc<RwLock<V>>>;
    fn read<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> Option<T>;
    fn write<T, F: FnOnce(&mut V) -> T>(&self, key: &K, f: F) -> Option<T>;
    fn insert(&self, key: K, value: V);
    fn remove(&self, key: &K);
}

// --- Tokio Implementation (for `std` and `tokio-runtime` feature) ---
// This uses `parking_lot`'s highly optimized locks and `std::sync::Arc`.

#[cfg(feature = "tokio")]
pub mod tokio {
    use super::*;
    
    use alloc::sync::Arc;
    
    use core::hash::Hash;
    use core::fmt::Debug;
    
    use derive_more::Deref;
    use derive_more::DerefMut;
    
    use hashbrown::HashMap;
    
    use parking_lot::RwLock;
    
    #[derive(Debug, Clone, Deref, DerefMut)]
    pub struct LocalStore<K, V> {
        data: Arc<RwLock<HashMap<K, Arc<RwLock<V>>>>>,
    }

    impl<K, V> LocalStore<K, V>
    where
        K: Eq + Hash,
    {
        pub fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    impl<K, V> Default for LocalStore<K, V>
    where
        K: Eq + Hash,
    {
        fn default() -> Self {
            LocalStore::new()
        }
    }

    impl<K, V> LocalStore<K, V>
    where
        K: Eq + Hash + Copy,
        V: Default,
    {
        pub fn get(&self, key: &K) -> Arc<RwLock<V>> {
            let mut data = self.data.write();
            data.entry(*key)
                .or_insert_with(|| Arc::new(RwLock::new(V::default())))
                .clone()
        }
    }

    impl<K, V> Store<K, V> for LocalStore<K, V>
    where
        K: Eq + Hash + Clone,
        V: Send + Sync,
    {
        fn get(&self, key: &K) -> Option<Arc<RwLock<V>>> {
            let map_guard = self.data.read();
            let arc_record = map_guard.get(key)?;
            Some(Arc::clone(arc_record))
        }
        
        fn read<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> Option<T> {
            let map_guard = self.data.read();
            let arc_record = map_guard.get(key)?;
            let record_guard = arc_record.read();
            Some(f(&record_guard))
        }
        
        fn write<T, F: FnOnce(&mut V) -> T>(&self, key: &K, f: F) -> Option<T> {
            let map_guard = self.data.read();
            let arc_record = map_guard.get(key)?;
            let mut record_guard = arc_record.write();
            Some(f(&mut record_guard))
        }
        
        fn insert(&self, key: K, value: V) {
            let arc_value = Arc::new(RwLock::new(value));
            self.data.write().insert(key, arc_value);
        }
        
        fn remove(&self, key: &K) {
            self.data.write().remove(key);
        }
    }
}

// --- Embassy Implementation (for `no-std` and `embassy-runtime` feature) ---
// This uses `spin`'s no-std compatible locks and `alloc::sync::Arc`.

#[cfg(feature = "embassy")]
mod embassy {
    use super::*;
    
    use alloc::collections::BTreeMap;
    use alloc::sync::Arc;
    
    use core::hash::Hash;
    
    use spin::RwLock;

    // Note: BTreeMap is used here since HashMap from `hashbrown` has a default
    // `hasher` that is not no-std compatible. BTreeMap is a solid no-std alternative.
    // However, if you're careful to use a no-std compatible Hasher, `hashbrown` is an option.
    pub struct EmbassyStore<K, V> {
        data: RwLock<BTreeMap<K, Arc<RwLock<V>>>>,
    }

    impl<K, V> EmbassyStore<K, V>
    where
        K: Ord,
    {
        pub fn new() -> Self {
            Self {
                data: RwLock::new(BTreeMap::new()),
            }
        }
    }

    impl<K, V> Store<K, V> for EmbassyStore<K, V>
    where
        K: Eq + Ord + Hash + Copy,
        V: Send + Sync,
    {
        fn get(&self, key: &K) -> Option<Arc<RwLock<V>>> {
            todo!()
        }
        
        fn read<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> Option<T> {
            let map_guard = self.data.read();
            let arc_record = map_guard.get(key)?;
            let record_guard = arc_record.read();
            Some(f(&record_guard))
        }

        fn write<T, F: FnOnce(&mut V) -> T>(&self, key: &K, f: F) -> Option<T> {
            let map_guard = self.data.read();
            let arc_record = map_guard.get(key)?;
            let mut record_guard = arc_record.write();
            Some(f(&mut record_guard))
        }

        fn insert(&self, key: K, value: V) {
            let arc_value = Arc::new(RwLock::new(value));
            self.data.write().insert(key, arc_value);
        }

        fn remove(&self, key: &K) {
            self.data.write().remove(key);
        }
    }
}
