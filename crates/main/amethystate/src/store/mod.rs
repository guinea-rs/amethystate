#[cfg(not(feature = "bench-internals"))]
pub(crate) mod backend;
#[cfg(feature = "bench-internals")]
pub mod backend;
pub mod builder;
pub mod config;
pub mod debouncer;
pub mod declared;
pub mod default;
pub mod durable;
mod error;
pub mod facts;
pub mod format;
pub mod inspector;
pub mod instances;
pub mod kv;
pub mod meta;
pub mod moved;
pub mod opening;
pub mod places;
mod primitives_factory;
pub mod reading;
pub mod rule;
mod rules;
pub mod screening;
mod state_slice;
pub(crate) mod sync_backend;
mod traits;
mod types;
pub mod writing;

pub use amethystate_core::path::{
    IntoStorePath, Key, Level, PathRef, SmolStr, StaticPath, StorePath, StorePathError, Stored,
    Under,
};
pub use amethystate_core::primitives::error::{WriteResult, WriteValue};
#[cfg(feature = "memory")]
pub use builder::Persistence;
pub use declared::{Declared, Holds};
pub use durable::{Commit, Durable};
pub use error::{IntoStorageReport, Occupied, StorageError, StorageResult, one_line};
pub use inspector::InspectorBackend;
pub use kv::{Cleared, Kv};
pub use opening::{Open, OpenStore, OpenStruct, Schema, WhenItWillNotRead, WillNotOpen};
pub use primitives_factory::*;
pub use reading::{LoadMap, LoadMapResult, ReadResult, ReadValue, ScanKeys, ScanResult};
pub use rule::{Invalid, Rule, RuleContext, refused, refused_or_default};
pub(crate) use rule::{judge_declared, load_declared, save_declared};
pub use rules::*;
pub use state_slice::*;
pub use traits::*;
pub use types::*;
pub use writing::{Flush, FlushResult, KvResult, KvWrite};
