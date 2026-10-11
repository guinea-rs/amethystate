use crate::Store;
use amethystate_core::ReactiveScope;
use amethystate_core::path::StorePath;

/// Where a declared struct's fields live, as the levels they are under.
pub trait StateScope {
    const PATH: StorePath;

    /// `PATH` as one string, for the places that need it before the program
    /// runs - a migration's dependency list, which is a `&'static [&'static
    /// str]`. The macro writes this and the joined half of `PATH` from the same
    /// token, so the two cannot come apart.
    const KEY: &'static str;

    /// The `id` the struct was declared with, `None` for the prefix's unnamed
    /// line. See [`Lineage`](crate::schema::Lineage).
    const ID: Option<&'static str> = None;
}

pub trait AmeStateSlice: Sized {
    /// Opens the struct through its [`Open`](crate::Open), or says why it
    /// would not open.
    fn try_load_slice(store: &Store) -> Result<Self, crate::store::opening::OpenStruct>;

    /// Opens the struct through its [`Open`](crate::Open).
    ///
    /// # Panics
    ///
    /// Where [`AmeStateSlice::try_load_slice`] answers `Err`, with what it said.
    #[track_caller]
    fn load_slice(store: &Store) -> Self {
        crate::store::opening::opened(Self::try_load_slice(store))
    }

    fn subscribe_all<F>(&self, callback: F) -> ReactiveScope
    where
        F: Fn() + Send + Sync + 'static;

    fn subscribe_all_external<F>(&self, callback: F) -> ReactiveScope
    where
        F: Fn() + Send + Sync + 'static;
}
