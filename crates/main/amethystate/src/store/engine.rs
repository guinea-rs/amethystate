use crate::migration::set::MigrationSet;
use crate::store::StorageResult;
use crate::store::builder::Backend;
#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
use crate::store::config::FileWritePolicy;
use crate::store::config::StoreConfig;
#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
use crate::store::opening::WhenItWillNotRead;
use crate::{MigrationReport, Store};

/// An engine and what it is set to, as
/// [`StoreBuilder::backend`](crate::StoreBuilder::backend) takes it.
///
/// A [`Backend`] makes one with every setting at its default. An engine that
/// has settings has a type of its own to carry them - [`Redb`], [`Text`],
/// [`Memory`] - so a setting can only be given to the engine that reads it.
///
/// ```
/// use amethystate::StoreBuilder;
/// # #[cfg(feature = "redb")]
/// use amethystate::store::engine::Redb;
///
/// # #[cfg(feature = "redb")] {
/// # let path = amethystate_core::test_utils::TempPath::new("doc_engine");
/// let store = StoreBuilder::new(&*path)
///     .backend(Redb::new().parallel_reads(true))
///     .build()
///     .unwrap();
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Engine {
    backend: Backend,
    settings: Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settings {
    Defaults,
    #[cfg(feature = "redb")]
    Redb(Redb),
    #[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
    Text(TextSettings),
    #[cfg(feature = "memory")]
    Memory(Memory),
}

impl Engine {
    /// Which engine this is.
    pub const fn backend(&self) -> Backend {
        self.backend
    }

    /// Opens a store on this engine directly, skipping the builder.
    ///
    /// [`StoreBuilder`](crate::StoreBuilder) is the ordinary route - it also
    /// collects migrations and settings this does not.
    pub fn open(
        self,
        mut config: StoreConfig,
        mset: MigrationSet,
    ) -> StorageResult<(Store, MigrationReport)> {
        self.settle(&mut config);
        self.backend.open(config, mset)
    }

    fn settle(self, config: &mut StoreConfig) {
        match (self.settings, config) {
            (Settings::Defaults, _) => {}
            #[cfg(feature = "redb")]
            (Settings::Redb(redb), config) => {
                config.redb_cache_size = redb.cache_size;
                config.parallel_reads = redb.parallel_reads;
            }
            #[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
            (Settings::Text(text), config) => {
                config.file_write = text.file_write;
                config.will_not_read = text.will_not_read;
            }
            #[cfg(feature = "memory")]
            (Settings::Memory(memory), config) => {
                config.parallel_reads = memory.parallel_reads;
            }
        }
    }
}

impl From<Backend> for Engine {
    fn from(backend: Backend) -> Self {
        Self {
            backend,
            settings: Settings::Defaults,
        }
    }
}

/// The `redb` engine, with its settings.
#[cfg(feature = "redb")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Redb {
    cache_size: usize,
    parallel_reads: bool,
}

#[cfg(feature = "redb")]
impl Redb {
    pub const fn new() -> Self {
        Self {
            cache_size: 0,
            parallel_reads: false,
        }
    }

    /// How many bytes of the file redb keeps in memory. None by default.
    ///
    /// What the store's fields, cells and maps hold is in memory already, so a
    /// cache of redb's own is a second copy of the same data, and it grows to
    /// the size of the file. Without one, the reads that go to the file - a
    /// `get`, a [`Kv`](crate::store::Kv) read, a scan - are served by the
    /// system's file cache instead: about 20 µs for a `get` on a store of a
    /// hundred thousand entries, against 2 µs from redb's cache. Worth setting
    /// for a store read mostly through `Kv`.
    pub const fn cache_size(mut self, bytes: usize) -> Self {
        self.cache_size = bytes;
        self
    }

    /// Lets reading a large collection back use more than one core. Off by
    /// default.
    ///
    /// Parsing every stored key and decoding every value is around four
    /// hundred milliseconds of a million-entry open, and dividing them takes
    /// that to about eighty. Off because this is a thread pool inside a state
    /// library, and an application that already has one should say whether it
    /// wants a second. Nothing is spawned while it is off - rayon builds its
    /// pool on first use. Below roughly a thousand entries the handing out
    /// costs more than the work, and the split does not happen there.
    pub const fn parallel_reads(mut self, yes: bool) -> Self {
        self.parallel_reads = yes;
        self
    }
}

#[cfg(feature = "redb")]
impl From<Redb> for Engine {
    fn from(redb: Redb) -> Self {
        Self {
            backend: Backend::Redb,
            settings: Settings::Redb(redb),
        }
    }
}

/// The in-memory engine, with its settings.
#[cfg(feature = "memory")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Memory {
    parallel_reads: bool,
}

#[cfg(feature = "memory")]
impl Memory {
    pub const fn new() -> Self {
        Self {
            parallel_reads: false,
        }
    }

    /// Lets reading a large collection back use more than one core. Off by
    /// default; see [`Redb::parallel_reads`] for what it trades.
    pub const fn parallel_reads(mut self, yes: bool) -> Self {
        self.parallel_reads = yes;
        self
    }
}

#[cfg(feature = "memory")]
impl From<Memory> for Engine {
    fn from(memory: Memory) -> Self {
        Self {
            backend: Backend::Memory,
            settings: Settings::Memory(memory),
        }
    }
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TextSettings {
    file_write: FileWritePolicy,
    will_not_read: WhenItWillNotRead,
}

/// A text engine - JSON, TOML or RON - with its settings.
///
/// The three keep the store the same way, in a file a person can open and
/// edit, so they take the same settings and differ only in the format.
#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    backend: Backend,
    settings: TextSettings,
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
impl Text {
    #[cfg(feature = "json")]
    pub fn json() -> Self {
        Self::of(Backend::Json)
    }

    #[cfg(feature = "toml")]
    pub fn toml() -> Self {
        Self::of(Backend::Toml)
    }

    #[cfg(feature = "ron")]
    pub fn ron() -> Self {
        Self::of(Backend::Ron)
    }

    fn of(backend: Backend) -> Self {
        Self {
            backend,
            settings: TextSettings::default(),
        }
    }

    /// How hard one write to the file fights before it reports a failure.
    ///
    /// This sits inside a single flush attempt, below
    /// [`Disk::retry_every`](crate::store::config::Disk::retry_every): only
    /// once it has run out does a flush count as having failed at all.
    pub fn file_write(
        mut self,
        configure: impl FnOnce(FileWritePolicy) -> FileWritePolicy,
    ) -> Self {
        self.settings.file_write = configure(self.settings.file_write);
        self
    }

    /// What a save does when the file it is about to replace will not read.
    ///
    /// The file is meant to be edited, which means it can be found
    /// half-typed, and a save that meets one cannot lay its own paths over a
    /// document it cannot parse. Without this the file is moved aside under
    /// `<name>.unreadable` and the save goes ahead: nothing is lost and the
    /// application keeps running.
    ///
    /// This is about a file that broke while the store was open. A file that
    /// will not read at the moment of opening is
    /// [`StoreBuilder::when_it_will_not_open`](crate::StoreBuilder::when_it_will_not_open).
    pub fn when_it_will_not_read(mut self, rule: WhenItWillNotRead) -> Self {
        self.settings.will_not_read = rule;
        self
    }
}

/// The text engine a [`Backend`] names, for a choice made while running;
/// any other engine is handed back.
#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
impl TryFrom<Backend> for Text {
    type Error = Backend;

    fn try_from(backend: Backend) -> Result<Self, Backend> {
        match backend {
            #[cfg(feature = "json")]
            Backend::Json => Ok(Self::of(backend)),
            #[cfg(feature = "toml")]
            Backend::Toml => Ok(Self::of(backend)),
            #[cfg(feature = "ron")]
            Backend::Ron => Ok(Self::of(backend)),
            #[allow(unreachable_patterns)]
            other => Err(other),
        }
    }
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
impl From<Text> for Engine {
    fn from(text: Text) -> Self {
        Self {
            backend: text.backend,
            settings: Settings::Text(text.settings),
        }
    }
}
