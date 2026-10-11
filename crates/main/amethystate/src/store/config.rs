use crate::store::builder::{Backend, Holds};
use crate::store::{StorageError, WhenItWillNotRead, WillNotOpen};
use error_stack::Report;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// A background flush's whole retry policy: how long to wait between
/// attempts, and how long a streak of failures may run before the store
/// says so out loud.
///
/// The store keeps trying until the flush lands or it is dropped, since a full
/// disk is usually someone deleting something in a minute.
#[derive(Clone)]
#[non_exhaustive]
pub struct RetryPolicy {
    pub interval: Duration,
    pub budget: Duration,
}

/// When the store touches the file, and what it does when the file will not
/// be touched.
///
/// Two-way on purpose. `debounce`, `retry_every` and `give_up_after` are
/// about writing; `watch_every` is about noticing somebody
/// else's write. Both are the same question - how this store and this file
/// keep up with each other - so they are one group rather than two.
///
/// Reached through [`StoreBuilder::disk`], which hands the defaults in and
/// takes the result back, so only what changes is written:
///
/// ```no_run
/// # use amethystate::store::builder::StoreBuilder;
/// # use std::time::Duration;
/// let store = StoreBuilder::new("settings.json")
///     .disk(|d| d.debounce(Duration::from_millis(200))
///                .watch_every(Duration::from_secs(1)))
///     .build()?;
/// # Ok::<(), error_stack::Report<amethystate::store::StorageError>>(())
/// ```
///
/// A named `fn(Disk) -> Disk` goes wherever the closure does, so a house style
/// shared by several stores is a function with a name rather than a value
/// passed around.
///
/// [`StoreBuilder::disk`]: crate::store::builder::StoreBuilder::disk
#[derive(Clone)]
#[non_exhaustive]
pub struct Disk {
    pub save_debounce: Duration,
    pub watch_debounce: Duration,
    pub retry_policy: RetryPolicy,
}

impl Disk {
    /// How long after the first unsaved write the buffer reaches the file.
    ///
    /// Writes that follow within that time join the same commit rather than
    /// putting it off, so writes that never pause are still saved once per
    /// window. Raising this batches more writes into one commit; lowering it
    /// narrows the window a crash can take. Neither affects reads, which see
    /// buffered writes immediately either way.
    pub fn debounce(mut self, every: Duration) -> Self {
        self.save_debounce = every;
        self
    }

    /// How long the file must sit still before a change made outside the
    /// process is read back.
    ///
    /// Nothing polls. The watcher is event-driven - inotify,
    /// ReadDirectoryChangesW, FSEvents - and this is the quiet period after the
    /// last event before the file is re-read, so an editor writing in several
    /// bursts is one re-read rather than several. Lowering it makes an external
    /// edit visible sooner; raising it costs nothing until somebody is writing
    /// the file continuously.
    pub fn watch_every(mut self, every: Duration) -> Self {
        self.watch_debounce = every;
        self
    }

    /// How long a failed background flush waits before trying again.
    ///
    /// A retry carries the same buffered changes, tried again. Nothing is lost
    /// between attempts.
    pub fn retry_every(mut self, every: Duration) -> Self {
        self.retry_policy.interval = every;
        self
    }

    /// How long a streak of failing flushes may run before the store says so
    /// out loud.
    ///
    /// The flush keeps being retried until it lands or the store is dropped, so
    /// a disk someone frees up heals it without a restart. This bounds how long
    /// that goes on quietly before the streak is logged at `error` and
    /// [`Store::on_persist_failure`](crate::Store::on_persist_failure) observers
    /// hear of it. Writes are never refused for it: they keep landing in
    /// memory, and only a caller waiting for the disk is told it failed.
    pub fn give_up_after(mut self, within: Duration) -> Self {
        self.retry_policy.budget = within;
        self
    }
}

/// How hard one step of a file write is fought before it is given up on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteAttempts {
    /// How many times the step is tried, the first one included. One means no
    /// retry at all.
    pub attempts: usize,

    /// How long to wait between them.
    pub pause: Duration,
}

impl WriteAttempts {
    /// Tried this many times, back to back until [`apart`] says otherwise.
    ///
    /// [`apart`]: WriteAttempts::apart
    pub const fn times(attempts: usize) -> Self {
        Self {
            attempts,
            pause: Duration::ZERO,
        }
    }

    /// How long to wait between them.
    pub const fn apart(self, pause: Duration) -> Self {
        Self { pause, ..self }
    }

    /// Tried once, reported the moment it fails.
    pub const fn once() -> Self {
        Self::times(1)
    }

    /// The longest a step spends waiting before the failure is reported.
    ///
    /// There is one pause fewer than there are attempts: the last failure is
    /// not slept on.
    pub const fn budget(self) -> Duration {
        Duration::from_nanos(self.pause.as_nanos() as u64 * self.attempts.saturating_sub(1) as u64)
    }
}

/// The two ways writing a file fails, and what each is worth waiting out.
///
/// A text engine writes the whole document to a file of its own and then
/// replaces the target with it, so a reader meets either the old file or the
/// new one. Those two steps fail for unrelated reasons and deserve unrelated
/// budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileWritePolicy {
    /// Getting the bytes into a file of their own. This is ordinary I/O: a
    /// full disk or a dead device stays that way, so a few quick attempts are
    /// all it is worth.
    pub write: WriteAttempts,

    /// Replacing the target with it. An antivirus, an indexer or a cloud
    /// client holding the file lets go on its own, so the same call succeeds a
    /// moment later - which is why this budget is the longer of the two, and
    /// why raising it costs a caller nothing until something really is stuck.
    pub replace: WriteAttempts,
}

impl FileWritePolicy {
    /// How hard getting the bytes into a file of their own is fought.
    pub const fn writing(self, attempts: WriteAttempts) -> Self {
        Self {
            write: attempts,
            ..self
        }
    }

    /// How hard replacing the target with it is fought.
    pub const fn replacing(self, attempts: WriteAttempts) -> Self {
        Self {
            replace: attempts,
            ..self
        }
    }
}

impl Default for FileWritePolicy {
    fn default() -> Self {
        Self {
            write: WriteAttempts::times(3).apart(Duration::from_millis(15)),
            replace: WriteAttempts::times(5).apart(Duration::from_millis(100)),
        }
    }
}

/// What a store refuses to hold.
///
/// Three separate things decide that:
///
/// - **The codec's ceiling is a fact.** `ron` will not read past 64 levels
///   whatever anyone configures. A write past it produces a file that codec
///   cannot read, so it is refused always, and there is nothing here to turn
///   that off. See [`Backend::depth_ceiling`].
/// - **Key depth is a setting**, and it is the one below. How deep a path may
///   go is the application's business - it knows the shape of its own paths -
///   and capping it also reserves the rest of the shared budget for values,
///   which is the half nobody thinks about: sixty levels of path on ron leaves
///   four for whatever is stored there.
/// - **Portability is a policy**, spelled as the set of engines the contents
///   must stay readable on. Today it settles depth alone, by lowering the
///   ceiling to the shallowest engine named. What a format can *express* -
///   non-finite floats on json, anything past `i64` on toml, a non-string map
///   key on every text engine - is a separate question no setting here answers.
///
/// [`Backend::depth_ceiling`]: crate::store::builder::Backend::depth_ceiling
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteLimits {
    /// How many levels a path may have, or no limit of the store's own.
    ///
    /// The codec's ceiling still applies either way, and the path spends the
    /// same budget the value does.
    pub key_depth: Option<usize>,

    /// Engines this store's contents must remain readable on, beyond the one
    /// actually running.
    ///
    /// Empty means no such claim, which is the default: a store nobody intends
    /// to move has no reason to pay for it.
    pub portable_across: Vec<Backend>,
}

impl WriteLimits {
    /// At most this many levels in a path.
    pub fn key_depth(mut self, levels: usize) -> Self {
        self.key_depth = Some(levels);
        self
    }

    /// Keep the contents readable on these engines, whichever one is running.
    ///
    /// This lowers the depth ceiling to the shallowest engine named, and
    /// settles four more properties the same way - non-finite floats, enums,
    /// a nested `Option` and an integer past `i64` - each held only if the
    /// running engine and every engine named holds it.
    ///
    /// The claim names its engines, so it stays what the application asked for
    /// when this crate gains another one, and so it can be as narrow as the
    /// requirement really is: an application shipping json on the desktop and
    /// sqlite on a phone needs those two and has no opinion about ron.
    pub fn portable_across(mut self, engines: impl IntoIterator<Item = Backend>) -> Self {
        self.portable_across = engines.into_iter().collect();
        self
    }

    /// The deepest a path and its value may go together, given the engine that
    /// is running and whatever else this store promised to stay readable on.
    pub fn ceiling(&self, running: Backend) -> usize {
        self.portable_across
            .iter()
            .map(|engine| engine.depth_ceiling())
            .chain(std::iter::once(running.depth_ceiling()))
            .min()
            .unwrap_or_else(|| running.depth_ceiling())
    }

    /// Whether a `NaN` or an infinity survives here and everywhere else this
    /// store promised to stay readable.
    ///
    /// The running engine counts for the same reason its ceiling does: a value
    /// its codec cannot read back is lost whatever anyone configured, so this
    /// is `false` on json and sqlite with nothing named at all.
    pub fn non_finite_floats(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::non_finite_floats)
    }

    /// Whether an enum survives here and everywhere else this store promised to
    /// stay readable.
    pub fn enums(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::enums)
    }

    /// The same for an integer wider than an `i64`, which toml has no room for.
    pub fn an_integer_past_i64(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::an_integer_past_i64)
    }

    /// The same for a `u128` or an `i128` of any value, which toml and ron have
    /// no type for.
    pub fn a_128_bit_integer(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::a_128_bit_integer)
    }

    /// The same for an integer neither an `i64` nor a `u64` holds, which json
    /// reads back as neither.
    pub fn an_integer_past_64_bits(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::an_integer_past_64_bits)
    }

    /// The same for `Some(None)`, which every engine but ron reads back as
    /// `None`.
    pub fn a_nested_option(&self, running: Backend) -> bool {
        self.everywhere(running, Holds::a_nested_option)
    }

    /// Whether the engine running and every engine named beside it all hold
    /// what `ask` asks about. A value one of them would alter is refused by all
    /// of them, which is what naming them was for.
    fn everywhere(&self, running: Backend, ask: fn(Holds) -> bool) -> bool {
        ask(running.holds())
            && self
                .portable_across
                .iter()
                .all(|engine| ask(engine.holds()))
    }
}

/// A flush that has been failing longer than its budget, and what it was
/// carrying.
///
/// `report.current_context()` says which failure it is -
/// [`StorageError::Flush`] for the disk, [`StorageError::Codec`] for the
/// document - and `{report:#}` renders it when that is what is wanted.
///
/// `non_exhaustive` because this is handed *to* a caller's callback: what is
/// worth telling them about a failing streak - how long it has run, how many
/// attempts it has cost - is the sort of thing that gets added, and a caller
/// reads the fields they want rather than destructuring the lot.
#[non_exhaustive]
pub struct GaveUp<'a> {
    pub why: &'a Report<StorageError>,

    /// Every path written since the last flush that landed.
    ///
    /// Candidates rather than culprits: a document is rendered whole and a
    /// render that fails names no node, so what can be said is which paths
    /// were waiting on it. The writer knows what it wrote, and a list it can
    /// look at beats a failure that names nothing.
    pub unsaved: &'a [amethystate_core::path::StorePath],
}

/// What an observer of a store's background saving hears.
///
/// Observers decide nothing: writes keep landing in memory whatever the disk
/// does, and the store keeps retrying. An observer is how an application
/// finds out, to say so to its user.
#[non_exhaustive]
pub enum PersistEvent<'a> {
    /// A failing streak outlived the retry budget.
    GaveUp { failure: &'a GaveUp<'a> },

    /// A save landed after a streak that gave up.
    Recovered,
}

/// An observer of a store's background saving.
pub type PersistObserver = Arc<dyn Fn(&PersistEvent<'_>) + Send + Sync>;

/// Everything a store is opened with, as
/// [`StoreBuilder`](crate::StoreBuilder) settles it.
///
/// `non_exhaustive`, and the most load-bearing one in this crate: a store gains
/// a setting now and then, and without this every one of those is a major
/// version for code that wrote the struct out by hand.
/// [`StoreBuilder`](crate::StoreBuilder) is the way to build one, and
/// [`StoreConfig::new`] the way to start from the defaults.
#[non_exhaustive]
pub struct StoreConfig {
    pub path: PathBuf,
    pub save_debounce: Duration,
    pub watch_debounce: Duration,
    pub retry_policy: RetryPolicy,

    /// What one write to one file is worth, as against [`retry_policy`], which
    /// is what a flush is worth once the write under it has already failed.
    ///
    /// [`retry_policy`]: StoreConfig::retry_policy
    pub file_write: FileWritePolicy,

    /// What this store refuses to hold, as against what its codec cannot.
    pub limits: WriteLimits,

    /// Whether reading a large collection back may use more than one core.
    ///
    /// Parsing every stored key and decoding every value is around four
    /// hundred milliseconds of a million-entry open, and dividing them takes
    /// that to about eighty. Off by default: this is a thread pool inside a
    /// state library, and an application that already has one should say
    /// whether it wants a second. Nothing is spawned while this is false -
    /// rayon builds its pool on first use.
    ///
    /// Small collections are unaffected either way. Below roughly a thousand
    /// entries the handing out costs more than the work, and the split does
    /// not happen there.
    ///
    /// Read by the `redb` and memory engines. `sqlite`, the text engines and
    /// `localstorage` take `StoreBackend::parallel_reads`'s default of
    /// `false`, so asking them for it is accepted and changes nothing about how
    /// they read. In a page there is one thread, and the work stays on it.
    pub parallel_reads: bool,

    /// What an open does when the store's own files will not read.
    ///
    /// Read by the engine, at the point it finds out - which is before it has
    /// looked at anything else, so what starting fresh takes away is the store
    /// as the last run left it and nothing this one wrote.
    pub will_not_open: WillNotOpen,

    /// What a save does when the file it is about to replace will not read.
    ///
    /// Read by the text engines only: theirs is the file somebody else edits.
    pub will_not_read: WhenItWillNotRead,
}

impl StoreConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            save_debounce: Duration::from_millis(300),
            watch_debounce: Duration::from_millis(500),
            retry_policy: RetryPolicy {
                interval: Duration::from_secs(5),
                budget: Duration::from_secs(60),
            },
            file_write: FileWritePolicy::default(),
            limits: WriteLimits::default(),
            parallel_reads: false,
            will_not_open: WillNotOpen::default(),
            will_not_read: WhenItWillNotRead::default(),
        }
    }
}
