---
title: Configuring a store
sidebar:
  order: 13
---

Everything here is set on the `StoreBuilder` before `build`, and applies to the
whole store. Every one has a default that is meant to be left alone; this page
is for the cases where it is not.

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

Everything about when this store touches its file lives under
`StoreBuilder::disk`, which hands the defaults in and takes back what you
changed - so only the settings you name are written, and forgetting one cannot
zero the rest.

`Disk::debounce` is how long after the first unsaved write the flush comes.
Writes made in the meantime join that flush instead of putting it off, so a
stream that never pauses is still written once per window. Raising it batches
more writes into one commit; lowering it narrows the window
a crash can take. Reads are unaffected either way - a buffered write is visible
at once.

The flush the window brings commits the whole buffer, on every engine. A
durable write asks for a flush of the path it wrote, and how far that one
reaches is the engine's answer rather than this setting's: `redb` and `sqlite`
commit what is buffered at that path and below it, a text engine rewrites the
whole file and so commits everything waiting.
[Durability](/amethystate/concepts/durability/) has the consequences.

`Disk::watch_every` is the other direction: how long the file has to sit still
before a change made outside the process is read back. Nothing polls - the
watcher is event-driven, on inotify, `ReadDirectoryChangesW` or FSEvents
depending on the platform - and this is the quiet period after the last event,
so an editor saving in several bursts costs one re-read instead of several.

`watch_every` is for the text engines only, the ones a person might edit by
hand. `debounce` applies to every engine.

## A flush that keeps failing

A failing flush is not a lost write. The same buffered changes are retried, and
they keep being retried until they land or the store is dropped - a full disk
that someone frees up heals the store with no restart.

What is configurable is how often it tries, and how long it goes on quietly:

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

`Disk::retry_every` is the gap between attempts. `Disk::give_up_after` is how
long a failing streak may run before the store says so: once, at `error` under
the `amethystate` target, and to every observer of its saving. It is how long
the failing goes on *quietly*, never how long it goes on.

Writers are never told. A write lands in memory whatever the disk is doing, the
buffer keeps it, and freeing space heals the store minutes or hours later with
no restart. A full disk is an ordinary Tuesday on plenty of machines, and a
field write has nothing to do about it - the application does, once, in one
place. Only a caller that asked whether its write reached the disk hears the
failure: `durable()`, `save_now`, a flush awaited.

### Hearing about it

Any number of places can listen on a store that is already open - a status bar,
a telemetry sink, a test:

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

An observer hears every streak that outlived `give_up_after`, and then the save
that lands after it. It decides nothing. It stays attached while its watch is
kept, and dropping the watch detaches it. It runs on the debouncer's thread,
where closing the store is refused; one that panics is logged, and the store
saves on.

`failure.why.current_context()` says which failure it is. `StorageError::Flush`
is the disk: full, read-only, taken away, held by something else.
`StorageError::Codec` is the document not rendering - a failure that will not
heal on its own, the same on the hundredth attempt as on the first.
`failure.unsaved` is every path written since the last flush that landed:
candidates rather than culprits, since a document is rendered whole and a render
that fails names no path.

It arrives as a `Report<StorageError>` rather than as one of the
[error sets](/amethystate/concepts/errors/), and deliberately: this runs on the debouncer's
thread with nobody's call to answer, so it is the engine's report rather than a
boundary's.

`Store::persist_failure` is the same news for a caller that would rather ask
than listen: the failure the last streak gave up with, until a save lands
again.

Most codec failures never get this far. A value is encoded where it is written,
so one the format cannot hold is refused by `set` itself and never enters the
buffer.

## One write to one file

Below the retry budget, and not beside it: this is what happens *inside* a
single attempt, and only once it runs out does a flush count as having failed
at all.

<!-- shown: how hard one write to one file fights -->
```rust
let store = StoreBuilder::new(settings)
    .file_write(|w| {
        w.writing(WriteAttempts::times(3).apart(Duration::from_millis(50)))
            .replacing(WriteAttempts::times(20).apart(Duration::from_millis(250)))
    })
    .build()?;
```
<!-- /shown -->

It applies to the text engines, which write a whole document to a file of its
own and then replace the target with it. Those two steps fail for unrelated
reasons and take separate budgets: `writing` is ordinary I/O, where a full disk
stays full and a few quick attempts are all it is worth, while `replacing` is
an antivirus or a cloud client holding the file, which lets go on its own - so
its budget is the longer one, and raising it costs nothing until something
really is stuck.

redb and SQLite hold their own handle and write through it, so there is no
replacement to block.

## What the store refuses to hold

Set through `limits`, and described on its own page:
[What a store refuses to hold](/amethystate/store/limits/).

## Reading a large collection

<!-- shown: letting a large read use more than one core -->
```rust
let store = StoreBuilder::new(settings).parallel_reads(true).build()?;
```
<!-- /shown -->

Parsing every stored key and decoding every value is around four hundred
milliseconds of a million-entry open; splitting the work takes that to about
eighty. Off by default, because this is a thread pool inside a state library
and an application that already has one should say whether it wants a second.
While it is off nothing is spawned - the pool is built on first use.

Below roughly a thousand entries the handing out costs more than the work, and
the split does not happen.

The redb engine is the one that reads this setting. SQLite and the text engines
keep their reads on the calling thread, so asking them for it is accepted and
changes nothing.

## What is not a setting

`migrations`, `provide` and `context` sit on the same builder and are not
configuration. They are **inputs**: the steps to run, the values those steps
are handed, and the values the declared rules are handed.
[Migrations](/amethystate/migrations/overview/) covers the first two, and
[Rules](/amethystate/state/rules/) the third.

The two that hand over a value are separate because they are read from
different places. A migration step runs once, inside `build`, on the thread
that called it, so `provide` takes anything at all - an `Rc`, a handle its
toolkit refuses to move. A rule runs every time a value arrives or is written,
including from the thread watching the file, so `context` asks for
`Send + Sync`.
