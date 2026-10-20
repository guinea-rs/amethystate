---
title: Configuring a store
sidebar:
  order: 13
---

Configure the store before opening it. Shared options belong on `StoreBuilder`;
engine-specific options belong on `Redb`, `Text` or the corresponding engine
configuration passed to `backend`. This prevents setting options an engine
cannot use.

Defaults cover ordinary use. The sections below explain when to adjust them.

## Timing

<!-- shown: how long a write waits, and how long an outside edit settles -->
```rust
let store = StoreBuilder::new(settings)
    .disk(|d| {
        d.debounce(Duration::from_millis(500))
            .watch_every(Duration::from_secs(2))
    })
    .build()?;
```
<!-- /shown -->

`StoreBuilder::disk` starts with default disk settings and lets you change only
the options you need.

`Disk::debounce` is the interval from the first pending write to its flush.
Later writes join that batch without restarting the timer, so a continuous
stream is still flushed regularly. Longer intervals batch more changes;
shorter intervals reduce the changes a crash can lose. Reads see buffered
writes immediately in either case.

A timed flush commits the whole buffer. A durable write asks to flush its path:
redb and SQLite commit that path and descendants, while text engines rewrite
the document and commit every pending write. See
[Durability](/amethystate/concepts/durability/).

`Disk::watch_every` controls reloading external edits in text engines. It is a
quiet period after the last file event, not a polling interval. This combines
an editor's burst of filesystem events into one reload. The watcher uses the
platform's filesystem notifications.

## A flush that keeps failing

A failed flush leaves its changes buffered. The store retries until saving
succeeds or it is dropped. A temporary full disk can therefore recover without
restarting the application.

Configure retry frequency and when the application should be notified:

<!-- shown: how long a failing flush stays quiet -->
```rust
let store = StoreBuilder::new(settings)
    .disk(|d| {
        d.retry_every(Duration::from_secs(2))
            .give_up_after(Duration::from_secs(30))
    })
    .build()?;
```
<!-- /shown -->

`Disk::retry_every` sets the interval between attempts. `Disk::give_up_after`
sets how long failures may continue before an error is logged and observers
are notified. Despite its name, it does **not** stop retries.

Ordinary buffered writes do not return disk errors. Handle persistence problems
centrally through an observer, or use `durable()`, `save_now` or an awaited flush
when a caller needs the saving result.

### Hearing about it

Attach observers to an open store for a status indicator, telemetry or a test:

<!-- shown: watching a store's saving from anywhere -->
```rust
let watch = store.on_persist_failure(|event| match event {
    PersistEvent::GaveUp { failure } => match failure.why.current_context() {
        StorageError::Flush => eprintln!("the disk is not taking writes: {:#}", failure.why),
        _ => eprintln!("not saved: {:#}", failure.why),
    },
    PersistEvent::Recovered => eprintln!("saved again"),
    _ => {}
});

if let Some(why) = store.persist_failure() {
    eprintln!("the last save that gave up: {why:#}");
}
```
<!-- /shown -->

An observer receives a failure after `give_up_after`, then a recovery event
when saving succeeds. Keep its watch to remain subscribed. The callback runs
on the debouncer thread, where closing the store is not allowed. A callback
panic is logged without stopping future saves.

`failure.why.current_context()` identifies the error category:

- `StorageError::Flush`: a storage operation failed, for example because the
  disk is full, read-only or temporarily inaccessible.
- `StorageError::Codec`: the complete document could not be encoded. Retrying
  unchanged data is unlikely to fix that.

`failure.unsaved` lists pending paths, not necessarily the path that caused the
failure: a document-level encoding error may not identify one value.

Observers receive a `Report<StorageError>`, preserving the engine's diagnostic
context. `Store::persist_failure` lets you query the last reported failure
instead; it clears when saving succeeds.

Most unsupported values fail at the write itself, before entering the buffer.
These observers concern failures during persistence.

## One write to one file

Text engines have additional retries **inside** a single flush attempt.
Only after these are exhausted does the outer flush retry policy apply:

<!-- shown: how hard one write to one file fights -->
```rust
let store = StoreBuilder::new(settings)
    .backend(Text::json().file_write(|w| {
        w.writing(WriteAttempts::times(3).apart(Duration::from_millis(50)))
            .replacing(WriteAttempts::times(20).apart(Duration::from_millis(250)))
    }))
    .build()?;
```
<!-- /shown -->

A text engine first writes a temporary document, then replaces the destination.
`writing` and `replacing` configure separate retry budgets for these stages.
A full disk will not usually recover after a few milliseconds, while a file
held by an antivirus or sync client may soon become replaceable. Replacement
therefore has the longer default budget.

redb and SQLite use database handles rather than this file-replacement protocol;
these settings do not apply to them.

## What the store refuses to hold

Use `limits` to restrict depth and portability. See
[Store limits](/amethystate/store/limits/).

## Reading a large collection

<!-- shown: letting a large read use more than one core -->
```rust
let store = StoreBuilder::new(settings)
    .backend(Redb::new().parallel_reads(true))
    .build()?;
```
<!-- /shown -->

Parallel decoding can reduce the cost of opening large collections. It uses a
lazily initialized thread pool and is disabled by default, so applications can
choose whether the extra pool is appropriate. Small collections stay on the
calling thread to avoid scheduling overhead.

This option is available for redb and the memory engine. SQLite and text
engines decode on the calling thread.

## redb's cache

<!-- shown: giving redb a cache of its own -->
```rust
let store = StoreBuilder::new(settings)
    .backend(Redb::new().cache_size(64 << 20))
    .build()?;
```
<!-- /shown -->

redb's own cache is disabled by default. Fields, cells and maps already keep
their live state in memory; another cache can duplicate that data. Direct
store reads without it rely on the operating system's file cache.

A workload dominated by `Kv` reads or scans may benefit from a redb cache.
Set its capacity in bytes and measure with the application's data; this changes
read performance, not the durability of commits.

## What is not a setting

The builder also accepts application inputs:

- `migrations`: the steps to run;
- `provide`: dependencies made available to migration steps;
- `context`: dependencies available to field rules and custom construction.

Migration dependencies may include non-`Send` values such as `Rc`: steps run
synchronously during `migrate` on its calling thread. Rules can also run on a
file-watcher thread, so values supplied through `context` must be `Send + Sync`.
See [Migrations](/amethystate/migrations/overview/) and
[Rules](/amethystate/state/rules/).
