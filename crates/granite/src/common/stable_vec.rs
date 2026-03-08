#![allow(unused)]

use super::Id;

/// ID-addressable collection for values that need stable handles.
///
/// Use this when you want "store once, pass around `Id`" behavior. Unlike
/// positional indexing, inserting new values does not shift existing handles.
pub struct StableVec<T> {
    data: generational_arena::Arena<T>,
}

impl<T> StableVec<T>
where
    T: 'static,
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

    /// Gets a value by [`Id`].
    ///
    /// Returns `None` when the ID is stale or belongs to a removed value.
    #[inline]
    pub fn get(&self, id: Id) -> Option<&T> {
        Self::verify_id(id);
        self.data.get(id.0)
    }

    /// Gets a mutable value by [`Id`].
    ///
    /// Returns `None` when the ID is stale or belongs to a removed value.
    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        Self::verify_id(id);
        self.data.get_mut(id.0)
    }

    /// Inserts a value and returns its stable [`Id`].
    #[inline]
    pub fn push(&mut self, value: T) -> Id {
        Id(
            self.data.insert(value),
            #[cfg(debug_assertions)]
            Self::type_id(),
        )
    }

    /// Removes a value by [`Id`].
    ///
    /// Returns the removed value when the ID is still valid.
    #[inline]
    pub fn remove(&mut self, id: Id) -> Option<T> {
        Self::verify_id(id);
        self.data.remove(id.0)
    }

    #[inline]
    #[cfg(debug_assertions)]
    fn verify_id(id: Id) {
        debug_assert!(id.1 == Self::type_id());
    }

    #[inline]
    #[cfg(not(debug_assertions))]
    fn verify_id(_id: Id) {}

    /// Iterates over all stored values, yielding `(Id, &T)` pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.data.iter().map(|(index, value)| {
            (
                Id(
                    index,
                    #[cfg(debug_assertions)]
                    Self::type_id(),
                ),
                value,
            )
        })
    }

    #[cfg(debug_assertions)]
    const fn type_id() -> std::any::TypeId {
        std::any::TypeId::of::<T>()
    }
}

impl<T> Default for StableVec<T> {
    fn default() -> Self {
        Self {
            data: generational_arena::Arena::default(),
        }
    }
}
