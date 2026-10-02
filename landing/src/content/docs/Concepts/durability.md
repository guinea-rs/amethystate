---
title: "Durability: availability over consistency"
sidebar:
  label: Durability
  order: 18
---

Most storage documentation opens with what it guarantees. This page opens with what it gives up, because that is the part you need in order to decide whether `amethystate` suits what you are building.

The parallel with CAP is loose but honest: faced with the same kind of choice, this library gives consistency away. Durability is what drags asynchrony — or blocking — into the paths that touch state, and a user interface wants neither. Nobody wants to await a read, and few want to await a write. Consistency here means the agreement between what you read and what is durably stored, and that agreement is what pays for keeping both paths plain synchronous calls. What you read is always the truth about your application's state. It is not always the truth about what is on disk.

Everything below is the shape of that decision: what it buys, what it costs, and where you can buy the guarantee back when you need it.

## How it works

Writes do not reach disk when you make them. `field.set()` puts the value in an in-memory buffer, notifies subscribers, and returns. A debounce timer flushes the buffer to storage a little later, and a burst of writes to one path costs one flush rather than one each.

**An identical write stops at the comparison.** The bytes go against what the store holds, buffered or committed, and a match ends it there: nothing buffered, no subscriber called, no flush scheduled. A slider that rounds to the step it was already on, a form saving on blur without an edit, a cache revalidated on a timer — each costs one memcmp. The comparison is on bytes, so a `f64` holding `NaN` deduplicates against itself.

**Subscribers hear about changes.** Writing the same value again is silent, so anything that needs to say *checked, still valid* needs its own place to say it.

Reads are cheap for the same reason. `get()` looks in that buffer first and answers from it, so a value you have been writing every frame is read back from memory, not from storage.

## What you get

**Reads see your own writes.** A value you just wrote is immediately visible through `get()`, even before it reaches disk. The buffer is consulted first.

<!-- shown: a write you can read and the disk cannot -->
```rust
state.port().set(9090)?;

let reads_back = state.port().get();
```
<!-- /shown -->

`reads_back` is `9090`, and the document on disk still holds the value from before.

**Flushes are atomic.** Everything buffered goes to storage in a single transaction. Storage never holds a half-written batch.

**Clean shutdown loses nothing.** Dropping the store flushes it. A process that exits normally has everything on disk.

**Except the global store, which nothing drops.** A static is never dropped, so a clean return loses the last debounce interval unless the guard goes out of scope or `amethystate::shutdown()` is called. See [Opening a store](/amethystate/store/opening/#writing-the-buffer-out).

## What you lose

**A crash loses the buffer.** A process killed by a signal, aborting on panic, or cut off by power loss loses everything written since the last flush. Destructors do not run in those cases.

**The window is the debounce interval** — 300 ms by default:

<!-- shown: narrowing the window -->
```rust
let store = StoreBuilder::new(path.path())
    .disk(|d| d.debounce(Duration::from_millis(50)))
    .build()?;
```
<!-- /shown -->

A smaller value narrows the window and flushes more often. A larger one widens it and flushes less. This is the only knob, and it controls exactly this trade.

**A notification does not mean the value is stored.** Subscribers are called during `set()`, before the flush. A subscriber can observe a value that a later crash erases. If your callback does something irreversible outside the process — sends a request, writes another file — do not treat the event as proof the value survived.

## What a field and the store agree on

Everything above is about the buffer and the disk, and there the disk is allowed to lag. Between a field and the store the answer is stricter: **what a field holds is what the store holds.** This is the guarantee the library is built around, and a place where it fails is a bug worth reporting.

- A write through a field lands in both or in neither. One an interceptor or a rule turns down, or the store refuses, leaves the field as it was.
- A change that arrives some other way - `Store::set` by path, a Tauri frontend, a person editing the file, a second store on it - reaches the field as the store took it.
- A declared rule that puts a value right writes the corrected value back, wherever the value came from, so the file never keeps a value the field does not show.
- `save` on a loaded struct leaves the struct in hand holding what it wrote.

### Where the two may differ

A few places, each on purpose, and each one a field can be asked about:

- **A value the field will not take.** Bytes that do not decode, a value a rule refuses, a default that could not be written because the path already held something else. The field shows its last good value or its default, and `try_get` answers `Err` saying why. The store keeps what it had, so nothing anyone wrote is destroyed before somebody fixes it.
- **A key removed under a field that keeps its value.** Under `on_delete = Keep`, the default, the field goes on showing the last value, and the store holds nothing there until the next write.
- **A correction the store will not take.** A rule put a stored value right, and writing the correction back failed - the store is closed, or the engine cannot hold what the rule made. The field holds the corrected value, the store the one it was given, and `try_get` answers `Reason::NotWrittenBack`.
- **A loaded struct between load and save.** It is plain data in your hands: assigning to it changes nothing until `save`, and an edit from outside never reaches it.
- **A save the store fails partway.** `save` judges every field before it writes any, so a rule's refusal writes none of them. A store error partway through - the store closed, a value the engine refuses - leaves the fields written before it, and `save` answers with the one it stopped at.
- **A closed store.** A field answers the last value it heard, and `try_get` says the store is closed.
- **A `volatile` field**, which is never stored at all.

### Concurrent access

The agreement is kept for one writer at a time. Two threads writing the same field are put in order by the store, and the field settles on the write the store took last. What it does not cover:

- **Read-modify-write is not atomic.** `update` and `modify` on a field or a cell, and `modify` and `upsert` on a map, read the value, run your closure and write the result. Two threads doing it at once can read the same value, and the second write drops the first one's change. The field and the store still agree - on the second write.
- **Nothing spans two fields.** Two threads writing related fields can leave a pair neither of them meant. A check between fields written where the struct is opened, with `open = manual`, runs there and once, not on every write.
- **Another process** on the same file is a matter for each engine, set out under [Closing](/amethystate/store/opening/#closing): redb refuses it, SQLite holds the file, and the text engines merge at save time, whoever saved last winning a key both wrote. Until the watcher brings its write in, the two processes disagree.

Where that matters, give the state one writer: one thread, or a lock of your own around the read and the write.

## Waiting for the disk

`save_now()` pushes the whole buffer out and returns once the store has
committed it.

<!-- shown: forcing everything out -->
```rust
state.port().set(9090)?;
store.save_now()?;
```
<!-- /shown -->

Fields, maps, cells and `Kv` each offer a `durable()` view: the same writes, every one of them returning only once the change is on disk. That keeps the guarantee to a single call, with no gap between writing and committing for you to be preempted in — or to forget:

<!-- shown: a write that waits for the disk -->
```rust
state.port().durable().set(9090)?;
```
<!-- /shown -->

Off the UI thread there is `set_async`, lazy like any future, so nothing happens — the write included — until it is awaited:

```rust
state.port().durable().set_async(9090).await?;
```

Reach for these at points where losing the last few hundred milliseconds actually matters — before launching an external process, after a step the user cannot repeat. Calling them on every write gives back the cheap writes you came for.

### A durable write commits its neighbours

`port` below is written durably, and `host` — buffered a moment earlier and never asked to be durable — is on disk when the call returns:

<!-- shown: what else a durable write commits -->
```rust
state.host().set("10.0.0.1".to_string())?;

state.port().durable().set(9090)?;
```
<!-- /shown -->

How wide that goes depends on the engine. A durable write puts its value in the buffer like any other and then flushes the path it wrote. The text engines rewrite the whole document on every commit, so one durable write makes the entire store durable — which is what happened above. `redb` and `sqlite` commit, in one transaction, what is buffered at that path and below it: a field's own value, every buffered entry of a map, one `Kv` path. There `host` would wait for the window. The async twins wait for the store's next full flush instead, so `set_async` commits everything buffered on every engine.

Two consequences worth holding on to. The cost of a durable write is not the cost of your value — it is the cost of whatever else that commit takes with it, which you did not choose and cannot see. And a value you deliberately left buffered can reach disk because something beside it was committed, so "not durable yet" is never a guarantee about where a value *is not*.

## There is no fast path

Nothing writes straight to disk. `durable()` waits for a flush rather than skipping the buffer, and no call skips it.

What you get for that is a thing you would otherwise have to check for yourself. The bookkeeping `amethystate` keeps — the mark saying a namespace was initialized, among others — goes into the same buffer a value does, so the two land together or not at all. A crash cannot leave you a namespace marked as initialized with nothing under it, or defaults written a second time over data that was already there.
