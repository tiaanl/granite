#![allow(unused)]

use std::collections::HashMap;
use std::hash::Hash;

use super::{Id, StableVec};

pub struct StableMap<K, V> {
    data: StableVec<V>,
    lookup: HashMap<K, Id>,
}

impl<K, V> StableMap<K, V>
where
    K: Eq + Hash,
    V: 'static,
{
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn get(&self, id: Id) -> Option<&V> {
        self.data.get(id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut V> {
        self.data.get_mut(id)
    }

    #[inline]
    pub fn push(&mut self, value: V) -> Id {
        self.data.push(value)
    }

    #[inline]
    pub fn get_id(&self, key: &K) -> Option<Id> {
        self.lookup.get(key).copied()
    }

    #[inline]
    pub fn get_by_key(&self, key: &K) -> Option<&V> {
        self.get_id(key).and_then(|id| self.data.get(id))
    }

    #[inline]
    pub fn insert_keyed(&mut self, key: K, value: V) -> Id {
        let id = self.data.push(value);
        self.lookup.insert(key, id);
        id
    }

    pub fn retain_keys<F>(&mut self, mut keep: F)
    where
        F: FnMut(&K) -> bool,
    {
        self.lookup.retain(|key, id| {
            if keep(key) {
                true
            } else {
                self.data.remove(*id);
                false
            }
        });
    }

    #[inline]
    pub fn get_or_insert_with<F>(&mut self, key: K, create: F) -> Id
    where
        F: FnOnce(&K) -> V,
    {
        if let Some(id) = self.get_id(&key) {
            return id;
        }

        let value = create(&key);
        self.insert_keyed(key, value)
    }
}

impl<K, V> Default for StableMap<K, V> {
    fn default() -> Self {
        Self {
            data: StableVec::default(),
            lookup: HashMap::default(),
        }
    }
}
