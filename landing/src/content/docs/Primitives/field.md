---
title: Fields
sidebar:
  order: 8
---

A field declared with `#[amethystate]` is a handle onto one path in the store.
Reading answers from memory, writing lands in memory at once and on disk on the
debounce.

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

A write answers nothing, because there is nothing for a caller to do about
what could stop it - see [what a write will not take](#what-a-write-will-not-take).
`try_get`'s `Err` is not a failure of the asking either - it is what this field
and the store do not agree about.

## Reading when the answer might not be the store's

`get` is the tolerant read: it always hands back something a render function
can draw. `try_get` is the same read with the doubt kept.

It answers `Err` when a change arrived that would not decode into this field's
type - a file edited outside the process, a migration that left something
behind, a codec that accepted a value it cannot read back - or when a declared
rule put a stored value right and the store would not take the correction.

The field goes on reporting the last value the store agreed with, and nothing
is delivered to subscribers. What is on screen was true a moment ago; the
declared default is a compile-time guess and the least likely thing the person
was looking at. So `get` keeps drawing, and `try_get` is where a caller that
cares finds out the store has stopped agreeing.

Nothing fails at the moment of asking. What failed happened earlier, and this
reports it - and answers `Ok` again as soon as a change decodes, so it holds
for exactly as long as it is true.

A value that was already unreadable when the struct opened is a different
moment, and one the declaration decides:
[When a value will not read](/amethystate/state/unreadable-values/).

## Writing

`update` and `modify` are read-modify-write and are **not atomic**. Two of them
racing on the same field can lose one of the two results, the same way two
`get`-then-`set` pairs would. Nothing in the store makes them atomic. Where that
matters, give the field one writer: one thread, or a lock of your own around the
read and the write -
[Concurrent access](/amethystate/concepts/durability/#concurrent-access).

`update` returns the value it computed, which saves the `get` you would
otherwise write on the next line. Where the field has a
[rule](/amethystate/state/rules/), what landed is what the rule left of it, and
`get` is the way to read that.

## What a write costs

Nothing waits for the disk. A write reaches the buffer, the subscribers hear
about it, and the flush happens on the debounce - so a field written every
frame costs a buffer write per frame and one commit per debounce interval.

To wait for the disk instead: [Durability](/amethystate/concepts/durability/).
Waiting can commit more than this one field, and how much more is the engine's
answer — the same page says which.

## What a write will not take

A field's declared [rule](/amethystate/state/rules/) refuses nothing: it puts
the value right, and the write lands as the rule left it.

Two things stop a write, and neither is a circumstance to handle at the call
site. A value the running engine's codec cannot encode, or a path deeper than
the store allows, is the declaration disagreeing with the engine: it fails the
same way on every run, so `set` panics naming the field, before the value
reaches the buffer. A store that was closed takes nothing: the write is
dropped, the field keeps its value, and a warning in the log says so.

Where a write has to answer - a value typed in by a person, an engine chosen at
run time - `durable()` is the write that does, with a `Result`.

## Where to go next

- Hearing about a change, on your own thread or somebody else's:
  [Subscriptions](/amethystate/concepts/subscriptions/).
- Collections whose keys are decided at run time:
  [ReactiveMap](/amethystate/primitives/reactive-map/).
