#![allow(unused)]

use std::{collections::HashMap, hash::Hash};

use super::{Id, StableVec};

/// Stable-ID set for the `key == value` case.
///
/// Use this when a value should be canonicalized to one stable [`Id`], such as
/// deduplicated layouts or descriptor values.
pub struct StableSet<T> {
    data: StableVec<T>,
    lookup: HashMap<T, Id>,
}

impl<T> StableSet<T>
where
    T: Eq + Hash + Clone + 'static,
{
    /// Returns `true` when no values are currently stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of currently stored values.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Gets a value by its stable [`Id`].
    #[inline]
    pub fn get(&self, id: Id) -> Option<&T> {
        self.data.get(id)
    }

    /// Gets a mutable value by its stable [`Id`].
    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.data.get_mut(id)
    }

    /// Returns the [`Id`] associated with `value`, if present.
    #[inline]
    pub fn get_id(&self, value: &T) -> Option<Id> {
        self.lookup.get(value).copied()
    }

    /// Returns the existing [`Id`] for `value`, or inserts it and returns a new one.
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
