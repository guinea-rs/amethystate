---
title: "Durability: memory and disk"
sidebar:
  label: Durability
  order: 18
---

Ordinary field writes update application state without waiting for disk.
A crash before the next flush can therefore lose recent changes.
Use explicit persistence operations when an action must not proceed until
its data is saved.

## How it works

`field.set()` updates the write buffer, notifies subscribers and returns.
A timer later flushes the accumulated changes. Multiple writes to one path
before that flush can be saved as one final value.

Identical encoded values are skipped: no new buffered write, notification or
flush is scheduled. Encoding and obtaining the previous value still cost work.
The comparison is on bytes, so an unchanged NaN representation can also be
recognized as identical.

Subscriptions report changes, not attempts. Send a separate event if callers
need to know that a check completed without changing anything.

`Field::get()` reads memory. Direct store reads check buffered writes before
backing storage.

## What you get

**Read your own writes immediately.** A successful buffered write is visible
through `get` before the file is updated:

<!-- shown: a write you can read and the disk cannot -->
```rust
state.port().set(9090);

let reads_back = state.port().get();
```
<!-- /shown -->

`reads_back` is `9090`; the document may still contain the previous value.

**Batched persistence.** Database engines commit their batches transactionally;
text engines replace documents and maintain recovery metadata for opening
rewrites. For text-file recovery limits, see [Files](/amethystate/store/files/).

**A final save attempt on shutdown.** Dropping the store flushes its buffer,
but saving can still fail. Call `save_now` or `close` explicitly if the
application must handle that error.

Keep the global-store guard or call `amethystate::shutdown()`; statics are not
automatically dropped. See [Opening a store](/amethystate/store/opening/#writing-the-buffer-out).

## What you lose

**An abrupt stop can lose pending writes.** Process termination, abort or power
loss may prevent destructors from running and discard changes since the last
successful flush.

The default batching interval is **300 ms**:

<!-- shown: narrowing the window -->
```rust
let store = StoreBuilder::new(path.path())
    .disk(|d| d.debounce(Duration::from_millis(50)))
    .build()?;
```
<!-- /shown -->

A shorter interval saves more often; a longer one batches more changes.
Disk failures can extend the unsaved period beyond that interval.

**Notification is not proof of persistence.** Callbacks run before the flush.
Do not use a change notification alone as permission for an irreversible
external action that depends on the value being saved.

## What a field and the store agree on

A field normally mirrors the store's current value in memory, even while the
disk is behind:

- A valid field write updates the store and the field. A closed store leaves
  the field unchanged; an unencodable value is rejected before buffering.
- Changes by path or external file edits update the field when processed.
- A rule's correction is written back to the store.
- Saving a loaded struct applies its rules and leaves corrected values in
  that struct.

A subscriber panic does not undo an accepted write: other subscribers are
notified and persistence remains scheduled before the panic reaches the writer.

### Where the two may differ

- **Unreadable data or an occupied path.** The field keeps its last readable
  value or default. `try_get` describes the problem; original stored data is
  preserved.
- **Deletion with `on_delete = Keep`.** The field retains its value while the
  store has no key, until a later write restores it.
- **A correction cannot be saved.** The field has the corrected value while
  storage retains the original; `try_get` reports `Reason::NotWrittenBack`.
- **Persistent-only structs.** Local assignments do not update storage until
  save, and external updates do not change the loaded struct.
- **A partially failed struct save.** All field rules run first, but writes
  are not one struct-wide transaction. A later error can leave earlier fields
  written.
- **Closed stores.** Cached fields remain readable; `try_get` reports closure.
- **Volatile fields.** These are intentionally never stored.

### Concurrent access

The store orders writes to the same field, but higher-level operations need
application coordination:

- `update` and `modify`, and a map's `modify` or `upsert`, are not atomic
  read-modify-write operations. Concurrent changes can overwrite each other.
- There is no transaction across related fields. Checks in a custom `Open`
  run at construction, not after every update.
- Another process may not see a write until its watcher processes it. Engine
  restrictions and text-file conflicts are described under
  [Closing](/amethystate/store/opening/#closing).

Use one writer or an application lock around a complete read-modify-write or
multi-field operation when consistency requires it.

## Waiting for the disk

`save_now()` flushes the whole buffer and returns its commit result:

<!-- shown: forcing everything out -->
```rust
state.port().set(9090);
store.save_now()?;
```
<!-- /shown -->

Fields, maps, cells and `Kv` provide `durable()` views. Their write methods
wait for persistence and return a `Result`, so the caller does not have to
separately remember a flush:

<!-- shown: a write that waits for the disk -->
```rust
state.port().durable().set(9090)?;
```
<!-- /shown -->

Use `set_async` when you want to await persistence without blocking the UI
thread. Like other futures, it does nothing, including the write, until polled:

```rust
state.port().durable().set_async(9090).await?;
```

Use these operations at meaningful boundaries: before starting an external
process, or before confirming work that cannot be repeated. Using them for
every slider update gives up the benefit of buffered writes.

### A durable write commits its neighbours

In this example, saving `port` durably also saves the buffered `host`:

<!-- shown: what else a durable write commits -->
```rust
state.host().set("10.0.0.1".to_string());

state.port().durable().set(9090)?;
```
<!-- /shown -->

The affected scope depends on the engine:

- Text engines rewrite the whole document, so a durable write commits all
  pending changes in that store.
- redb and SQLite flush the written path and its descendants. A durable field
  write need not commit an unrelated field; a durable map operation includes
  pending entries beneath its path.
- Async variants wait for the next whole-store flush on every engine.

Cost therefore depends on everything included in the flush, not just the one
value. Conversely, leaving a value buffered does not guarantee it is absent
from disk: another operation may flush it.

## There is no fast path

Durable writes still go through the buffer; they wait for its commit rather
than bypassing it. Related initialization metadata uses that buffer too,
so the engine can commit values together with the metadata describing them.
