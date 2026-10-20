---
title: Fields
sidebar:
  order: 8
---

A field declared with `#[amethystate]` is a handle to one stored value.
Reads use memory. Writes update memory immediately and are flushed to disk
after the configured interval.

<!-- shown: reading and writing a field -->
```rust
let port = state.port().get();

state.port().set(9090);

let raised = state.port().update(|port| port + 1);

state.port().modify(|port| *port += 1);
```
<!-- /shown -->

`get` returns the value itself. A field always has one: a path holding nothing
answers with the default its declaration gave it.

```rust
fn get(&self) -> T
fn try_get(&self) -> Result<T, Disagreement>
fn set(&self, value: T)
fn update<F: FnOnce(T) -> T>(&self, f: F) -> T
fn modify<F: FnOnce(&mut T)>(&self, f: F)
```

`set` returns `()`, not a `Result`; see
[what a write will not take](#what-a-write-will-not-take) for error handling.
`try_get` reports known problems such as an unreadable stored update or a
closed store. It does not perform a new disk read.

## Reading when the answer might not be the store's

`get` always returns a value, so a render function can keep displaying it even
if a later storage update cannot be read. Use `try_get` when the UI also needs
to show that problem.

For example, someone may edit the file and replace a number with text. The
field keeps its last readable value, does not notify subscribers of the invalid
update, and returns `Err` from `try_get`. Once a readable update arrives,
`try_get` returns `Ok` again.

`try_get` also reports cases such as a rule correcting a stored value when
the correction could not be written back. The error describes an existing
disagreement; reading the cache is not what caused it.

An unreadable value found while opening the struct is handled separately,
according to its declaration:
[When a value will not read](/amethystate/state/unreadable-values/).

## Writing

`update` and `modify` are **not atomic**: each reads a value, changes it and
writes the result. Concurrent calls can overwrite each other's results, just
like two `get`-then-`set` pairs. If this matters, use a single writer or protect
the complete read-modify-write operation with your own lock. See
[Concurrent access](/amethystate/concepts/durability/#concurrent-access).

`update` returns the value its closure computed. A field's
[rule](/amethystate/state/rules/) may normalize that value before storing it;
call `get` if you need the normalized result.

## What a write costs

Ordinary field writes do not wait for disk. They update the buffer and notify
subscribers; the store batches them into flushes. Updating a field every frame
therefore does not require committing every frame.

To wait for a write to reach disk, see [Durability](/amethystate/concepts/durability/).
Depending on the engine, that operation may also commit other pending writes.

## What a write will not take

A field's [rule](/amethystate/state/rules/) normalizes the value rather than
rejecting it. The normalized value is the one stored.

Disk failures do not reject a buffered write. If the disk is full or the file
cannot be replaced, the value remains in memory and the store retries saving.
[Store observers](/amethystate/store/configuration/#hearing-about-it) report
the problem. A subscriber error is logged at `error` and does not undo the write.

Other failures are handled differently:

- If the engine cannot encode the value, or the path exceeds the store's depth
  limit, `set` panics before buffering the write. The message identifies the field.
- If the store is closed, the write is ignored, the field keeps its previous
  value, and a warning is logged.

Use `durable()` when the caller needs a `Result` and must know whether a write
reached disk, for example before confirming an operation to the user.

## Where to go next

- Hearing about a change, on your own thread or somebody else's:
  [Subscriptions](/amethystate/concepts/subscriptions/).
- Collections whose keys are decided at run time:
  [ReactiveMap](/amethystate/primitives/reactive-map/).
