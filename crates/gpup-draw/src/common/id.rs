/// Stable handle used by all stable containers.
///
/// Use `Id` to reference values across frames or systems without depending on
/// positional indices.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(
    pub(crate) generational_arena::Index,
    /// The type of the data we are pointing to.
    #[cfg(debug_assertions)]
    pub(crate) std::any::TypeId,
);

impl std::fmt::Debug for Id {
    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (index, generation) = self.0.into_raw_parts();
        write!(f, "Id({index}:{generation})")
    }

    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (index, generation) = self.0.into_raw_parts();
        write!(f, "Id({index}:{generation}, {:?})", self.1)
    }
}
