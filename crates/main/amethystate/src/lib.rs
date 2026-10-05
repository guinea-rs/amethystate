//! Persistent reactive state for Rust GUI apps.

#![allow(clippy::complexity)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(
    not(any(
        feature = "redb",
        feature = "sqlite",
        feature = "json",
        feature = "toml",
        feature = "ron"
    )),
    allow(dead_code, unused_imports, unused_variables, unreachable_code)
)]
mod codec;
mod global;
mod macros;

pub mod migration;
pub mod observability;
pub mod reactive;
pub mod schema;
pub mod shape;
pub mod store;

pub type AmeData<T> = <T as AmeState>::Data;

/// What a migration step answers with: everything it can fail at, and nothing
/// else.
pub type MigrationResult<T> = crate::migration::StepResult<T>;

pub use erased_serde;
pub use error_stack;
pub use indexmap;
pub use inventory;
pub use serde;
pub use uuid;

pub use reactive::{
    AmeState, AmeStateNode, Change, Field, Id, MapChange, ReactiveCell, ReactiveMap,
    ReactiveMapKey, ReactiveMapValue, ReactiveScope, SignalSubscription,
};
pub use store::StoreSubscription;

pub mod errors {
    pub use crate::codec::CodecError;
    pub use crate::reactive::error::{
        FieldError, ReactiveFieldResult, ReactiveMapError, ReactiveMapResult, WriteResult,
        WriteValue,
    };
    pub use crate::store::StorageError;
    pub use amethystate_core::facts;
    pub use amethystate_core::failure::{Because, Caused};
    pub use error_stack::Report;
}
pub mod stores {
    pub use crate::store::default::*;
}

pub use store::{
    AmeStateSlice, Open, Schema, StateScope, StorageResult, StoreEvent, StoreOp, SubscriptionKind,
    builder::StoreBuilder, config::StoreConfig, default::Store,
};

pub use migration::{MigrationContext, MigrationError, MigrationPlan, MigrationReport};

pub use amethystate_macros::{amethystate, migrate};

pub mod prelude;
pub use global::*;

#[cfg(feature = "json")]
pub use serde_json;
pub use store::StoreBackend;
pub use store::StoreExt;

#[cfg(any(feature = "test-utils", test))]
pub mod test_utils;

#[cfg(feature = "async")]
pub mod client {
    pub use amethystate_core::AmeBackendAsync;
    pub use amethystate_core::AmeStateSliceAsync;
    pub use amethystate_core::async_impl::*;
    pub use amethystate_core::{FieldCore, ReactiveMapCore};

    use amethystate_core::async_impl::Field as CoreField;
    use amethystate_core::async_impl::ReactiveMap as CoreReactiveMap;

    pub type ReactiveMap<K, V, B> = CoreReactiveMap<K, V, B>;

    pub type Field<V, B> = CoreField<V, B>;
}
