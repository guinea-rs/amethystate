---
title: Opening a store
sidebar:
  order: 12
---

Create a `Store` with `StoreBuilder`, choosing a file location and storage
engine. Pass that store to your state structs or use it for direct path access.
Cloning a `Store` creates another handle to the same open store, not another
database connection or file.

An application typically keeps a store open until shutdown. You can also open
several stores with different files or engines.

## At a path you name

<!-- shown: opening a store at a path you name -->
```rust
let store = StoreBuilder::new(settings).build()?;
```
<!-- /shown -->

`build` returns a `Store` that you can pass to `new_with`. Dropping the store
attempts to flush pending writes, but cannot return a failure to your code.
Use explicit [Closing](#closing) if you need to handle that result.

## Or once, for the whole process

If you do not want to pass a store through your application, install a global
one and access it from anywhere in the process:

<!-- shown: opening the process-wide store -->
```rust
let _ame = "./app.redb".init_global();

let state = NetworkState::new();
```
<!-- /shown -->

`NetworkState::new()` uses the installed global store instead of accepting one
as an argument. Structs with `open = manual` omit this constructor and require
an explicit store; see [Custom construction](/amethystate/state/opening-structs/).
You can also access the store itself:

<!-- shown: reaching the global store from anywhere -->
```rust
let store = global_store();
store.kv().set("theme", &"dark".to_string())?;
```
<!-- /shown -->

**Keep the guard returned by `init_global`.** Dropping it closes the global
store. A standalone `"./app.redb".init_global();` therefore opens and immediately
closes it. Bind the guard in `main`, for example as `let _ame = ...`.

Installing another global store while one is open panics. After the first is
closed or shut down, another can be installed. Existing handles still refer
to the closed store; an old guard cannot close the replacement.
Accessing the global store before installation also panics.

For recoverable opening errors, such as a config the UI can offer to reset,
use `build_global`, which returns the error:

<!-- shown: opening the process-wide store without a panic -->
```rust
let _ame = match StoreBuilder::new("./app.redb").build_global() {
    Ok(guard) => guard,
    Err(InitGlobal::Open(why)) => {
        eprintln!("settings are unavailable: {why}");
        return Ok(());
    }
    Err(InitGlobal::AlreadyInstalled) => unreachable!("opened once, in main"),
};
```
<!-- /shown -->

Builder methods have global variants: `build_global`, `migrate_global`, and
both after `or_in_memory()`. They reject installation if a store is already
open. To install a store opened separately, use `install_global`; if occupied,
it returns the uninstalled store.

For migrations and global installation together:

<!-- shown: opening it with the migration pass -->
```rust
let (_ame, report) = StoreBuilder::new("./app.redb").migrate_global()?;

if report.has_drift() {
    eprintln!("a struct changed without a version bump");
}
```
<!-- /shown -->

## Letting the platform say where

Explicit paths suit tests and tools working on a chosen file. For an installed
application, use `located` to resolve an application directory:

<!-- shown: letting the platform say where the file goes -->
```rust
let config = StoreBuilder::located(|at| at.app("my-app", "settings"))?;

let named = StoreBuilder::located(|at| at.app_under(Layout::App, "my-app", "settings"))?;

let portable = StoreBuilder::located(|at| at.beside_the_executable("settings"))?;
```
<!-- /shown -->

Available location helpers:

| helper | location |
| --- | --- |
| `at.app("my-app", "settings")` | Application configuration directory using `Layout::App`. |
| `at.app_under(Layout::App, ..)` | Application directory under an explicitly selected layout. |
| `at.beside_the_executable("settings")` | Next to the executable, for portable installs. |

### Which convention `app` follows

`Layout` selects the directory convention:

| layout | convention |
| --- | --- |
| `Layout::App` | XDG-style layout on every platform, including Windows and macOS. |
| `Layout::Native` | The host platform's native application directories. |
| `Layout::ProjectDirs` | The layout produced by the `directories` crate. |

`app` defaults to `Layout::App`, not `Layout::Native`. Choose `Native` explicitly
if the application's files should follow the platform convention. Use
`ProjectDirs` to match an existing application using `directories`.

**Keep the chosen layout stable after release.** Switching conventions can make
the application look in another directory and appear to lose settings.
Use `app_under` to make the choice explicit.

### Moving a store that is already somewhere else

`located` calls your closure once and uses its returned `PathBuf`. The closure
can also detect or move existing files before returning the chosen location.

Move the complete engine layout, not just the supplied base path: a text store
also has metadata and may have recovery files. For example:

<!-- shown: moving every file a store is made of -->
```rust
fn relocate(from: &StoreLayout, to: &StoreLayout) -> std::io::Result<()> {
    for (old, new) in from.names().iter().zip(to.names()) {
        if old.exists() {
            std::fs::rename(old, new)?;
        }
    }

    Ok(())
}
```
<!-- /shown -->

`names` lists every possible layout path, including temporary files that may
not exist. Skip absent files when moving. Two layouts of the same engine list
corresponding roles in the same order; `present` lists only existing files.

Use `at.files_at` to resolve a layout without opening the old store:

<!-- shown: a store found where an older release left it -->
```rust
let store = StoreBuilder::located(|at| {
    let now = at.app(&app, "settings")?;
    let was = at.files_at(at.app(&app, "settings-legacy")?, backend);

    relocate(&was, &at.files_at(&now, backend)).ok();

    Ok(now)
})?
.backend(backend)
.build()?;
```
<!-- /shown -->

The application decides how to move: copy the files, rename them, ask the user,
or continue using the old location. The library only needs the final path.
Make this operation repeatable if a previous run might have stopped midway.

`beside_the_executable` is intended for writable portable installs. Installed
locations such as `Program Files`, `/usr/bin` or a signed macOS bundle are not
suitable for writing settings and can fail at opening.

## Which file the name becomes

A base path without an extension receives the engine's default extension:

<!-- shown: letting the engine name the file -->
```rust
let store = StoreBuilder::new(config_dir.join("settings"))
    .backend(Backend::Json)
    .build()?;
```
<!-- /shown -->

This creates `settings.json`. An explicit extension is preserved:

<!-- shown: naming the file yourself -->
```rust
let store = StoreBuilder::new(config_dir.join("settings.conf"))
    .backend(Backend::Json)
    .build()?;
```
<!-- /shown -->

This creates `settings.conf` containing JSON. The extension does not choose
the format: `backend` does. If you change the backend after setting an
extensionless path, the automatically chosen extension changes with it.

See [Installation](/amethystate/getting-started/installation/) for available
engines and formats.

## What to do when a value will not read, or its key is gone

Read failures and deleted keys have separate policies. By default, an unreadable
stored value prevents construction, while a deleted key leaves a live field
showing its previous value. To change those defaults:

```rust
use amethystate::store::OnUnreadable;

let store = StoreBuilder::new(path)
    .rules(|r| r.on_unreadable(OnUnreadable::UseDefault))
    .build()?;
```

Policies can be set on the store, struct or field. More specific declarations
take precedence: field, then struct, then store. A store-level `UseDefault`
therefore does not override an explicit `Refuse` on a structure.

For example, rebuildable thumbnail settings can use defaults on failure,
while a licence must remain readable:

<!-- shown: one process, two answers about the same store -->
```rust
#[amethystate(prefix = "thumbnails")]
pub struct Thumbnails {
    #[amestate(default = 0u32)]
    pub generated: u32,
}

#[amethystate(prefix = "licence", on_unreadable = Refuse)]
pub struct Licence {
    #[amestate(default = "".to_string())]
    pub holder: String,
}
```
<!-- /shown -->

With the store set to `UseDefault`, an unreadable thumbnail count uses its
default. An unreadable `Licence` still fails construction because it explicitly
uses `Refuse`. See [Unreadable values](/amethystate/state/unreadable-values/)
for policy inheritance and live-update behavior.

## What to do when the file itself will not read

An unreadable file is different from one unreadable field inside a valid
file. Policies also differ between opening and later saving.

### Opening

By default, opening an invalid database or unparseable document fails and leaves
the files untouched:

```rust
use amethystate::store::WillNotOpen;

let cache = StoreBuilder::new(path)
    .when_it_will_not_open(WillNotOpen::StartFresh)
    .build()?;
```

`StartFresh` deletes the store's layout files, including recovery copies,
then creates an empty store. A warning is logged before deletion, but the
operation is not reversible. Use it only for rebuildable data such as a cache.

It does not override permission failures, a file held by another process,
a SQLite database owned by another application, or a database requiring a
newer SQLite version. These are not reasons to delete someone else's data.

### Or running in memory

Enable the `memory` feature and add `or_in_memory()` before `build` or `migrate`
to continue with a fresh in-memory store if opening fails:

<!-- shown: running in memory when the file will not open -->
```rust
let (store, persistence) = StoreBuilder::new(path).or_in_memory().build();

if let Persistence::InMemory { because } = &persistence {
    eprintln!("settings will not be saved this run: {because}");
}
```
<!-- /shown -->

The result tells you whether persistence is available and why a fallback was
needed. The in-memory store starts empty, initializes structs from defaults
and does not overwrite the failed store's files. A failed migration component
is not converted into a successful migration by this fallback.

`Persistence::OnDisk` indicates a normal file-backed result. Global variants
`build_global` and `migrate_global` support the same fallback, but still reject
an already installed store.

For a deliberately temporary session, use `StoreBuilder::in_memory()` or
`.backend(Backend::Memory)` directly. The memory engine uses MessagePack values
per path; it does not reproduce a text file's document layout.

### Saving

A text file may become temporarily invalid while someone edits it. Saving must
then decide how to handle the unreadable document:

<!-- shown: what a save does with a file that will not read -->
```rust
let store = StoreBuilder::new(path)
    .backend(
        Text::json()
            .when_it_will_not_read(WhenItWillNotRead::TryAgainFor(Duration::from_secs(30))),
    )
    .build()?;
```
<!-- /shown -->

| policy | behavior |
| --- | --- |
| `TryAgainFor(duration)` | Retry without changing the file; after the duration, use `SetAside`. |
| `SetAside` | Move the file to `<name>.unreadable`, then save. |
| `Refuse` | Do not save while the file remains unreadable. |
| `Overwrite` | Replace it with the store's document, discarding the old contents. |

The default is `TryAgainFor(Duration::from_secs(5))`. The period starts at the
first save that encounters unreadable data, not at each retry. A successful
parse resets it. Pending writes remain in memory while saving is refused.

For user-edited settings, choose `Refuse` or the default to protect unfinished
edits. `Overwrite` is appropriate only when discarding the existing contents is
acceptable. `SetAside` preserves the unreadable file, but a later unreadable
copy replaces the previous one. For a symbolic link, the target is moved and
the link remains.

A refused save is a flush error: `save_now` returns it, while drop logs it.
These policies concern text documents, not redb or SQLite databases.

## Which migrations run

- `build` opens without executing migration steps. Schema registration and
  drift checking still occur.
- `migrate` executes the registered steps and returns a report. A failed
  component refuses opening with `OpenStore::Migrating` and that report.

After `.migrations()`, only the migration-opening methods remain available.
Use `or_in_memory()` before either opening path for fallback, or the
`build_global` / `migrate_global` variants for a global store.
See [Migrations](/amethystate/migrations/overview/).

## Writing the buffer out

Drop attempts a final flush but can only log failures. Call `save_now` while
the store is open if your code needs the result:

<!-- shown: writing the buffer out and hearing whether it landed -->
```rust
store.kv().set("port", &8080u16)?;

if let Err(report) = store.save_now() {
    eprintln!("settings were not saved: {report:?}");
}
```
<!-- /shown -->

`save_now` returns the flush result and leaves the store open. If it fails,
you can retry while the buffer is still available. A later drop attempts to
flush again and logs any failure.

## Closing

`close` flushes pending writes, stops background work and releases the store's
resources. Unlike `save_now`, it makes the store unusable for further storage
operations:

<!-- shown: closing the store and letting go of the file -->
```rust
if let Err(report) = store.close() {
    eprintln!("the last writes were not saved: {report:?}");
}
```
<!-- /shown -->

All clones refer to that same closed store. Direct reads, writes, scans and
flushes return their operation-specific `Closed` error. Closing it again is
allowed but does not retry the flush.

A field's `get` still returns its in-memory value; `try_get` reports
`Reason::Closed` to identify that cached state.

Close the store before handing its file to another process, moving it or
starting a coordinated backup. Resource release depends on the engine:

| engine | what closing releases |
| --- | --- |
| SQLite | Its database handle, allowing file operations previously blocked by it. |
| redb | Its exclusive open, allowing another store at that path. |
| JSON, TOML, RON | Background work; these engines hold no persistent file handle between flushes. |

**Use one process at a time for a text store.** Separate instances can merge
different data edits, but conflicting values use the last saved value.
Metadata is replaced as a whole, not merged; concurrent migrations also compete
for the same recovery files. This is not a multi-process coordination protocol.

**Global shutdown follows the same rules.** Dropping the guard closes its store
and logs failures. Call `amethystate::shutdown()` before dropping the guard to
receive the close result instead. Static storage is not dropped automatically;
without a guard or explicit shutdown, pending writes can be lost even on a
normal process exit.

A failed `close` still leaves the store closed. Retry with `save_now` **before**
closing if you need to recover from a saving failure. Another `close` call will
not recover the buffer.

For waiting on a single write, see
[Durability](/amethystate/concepts/durability/).
