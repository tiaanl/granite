#![allow(unused)]

use std::{collections::HashMap, hash::Hash};

use super::{Id, StableVec};

pub struct StableSet<T> {
    data: StableVec<T>,
    lookup: HashMap<T, Id>,
}

impl<T> StableSet<T>
where
    T: Eq + Hash + Clone + 'static,
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
    pub fn get(&self, id: Id) -> Option<&T> {
        self.data.get(id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.data.get_mut(id)
    }

    #[inline]
    pub fn get_id(&self, value: &T) -> Option<Id> {
        self.lookup.get(value).copied()
    }

    #[inline]
    pub fn get_or_insert(&mut self, value: T) -> Id {
        if let Some(id) = self.get_id(&value) {
            return id;
        }

        let id = self.data.push(value.clone());
        self.lookup.insert(value, id);
        id
    }
}

impl<T> Default for StableSet<T> {
    fn default() -> Self {
        Self {
            data: StableVec::default(),
            lookup: HashMap::default(),
        }
    }
}
