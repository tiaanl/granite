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
        Self::verify_id(id);
        self.data.get(id.0)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        Self::verify_id(id);
        self.data.get_mut(id.0)
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Id {
        Id(
            self.data.insert(value),
            #[cfg(debug_assertions)]
            Self::type_id(),
        )
    }

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
