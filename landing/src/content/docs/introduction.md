---
title: Introduction
sidebar:
  order: 1
---

I don't believe in GUI in Rust. To anyone who wants to build an interface rather
than reinvent the wheel, I say: take Tauri or Electron.

I started with the question "what do I write a GUI in?" and, without much
deliberation, arrived at Slint. If all you need is to draw the interface, Rust is
a tolerable choice. But you outgrow that need quickly, and behind it stands a
scatter of problems that matter just as much: where the state lives, how the app
updates, and a dozen smaller things. Tauri has answers for them; the native
desktop has none at all.

This library is my answer, one of a dozen possible, to the question "how do I
keep the sidebar's width between launches?". I read the sources of Halloy,
Sniffnet and smaller projects. Each solves it on the spot, for its own needs, and
to my mind clumsily. So I decided to strain a little — that is, to strain the
GPUs on Anthropic's servers — and solve it more systematically.

## What it is

`amethystate` saves application state between runs and notifies your code when
it changes. You declare the state as Rust structs; the library provides field
handles, storage and migrations.

A **store** manages the saved data, buffered writes and subscriptions. Open it
when the application starts, then use it to create your state structs.

```rust
#[amethystate(prefix = "network")]
pub struct NetworkState {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```

After `state.port().set(9090)`, reads return `9090` and subscribers to `port`
have been notified. Saving to disk happens separately.

Subscriptions can target a field, a map key or a path. A UI component watching
`port` does not receive changes to `host`, so it can update only when the value
it displays changes.

## What it decides, so you do not

**File location and format.** You can open a store by name in the platform's
application-data directory or provide a path yourself. Its **engine** determines
how the data is stored: redb, SQLite or a text document.

**When to save.** File writes are buffered in memory and flushed after a
configurable interval from the first unsaved change. Dragging a slider therefore
does not require a disk write on every frame. Call `save_now` when you need to
wait for all pending writes to reach disk. Closing the store also flushes them.

**Value encoding.** redb stores values as MessagePack, SQLite stores them as
JSON, and the text engines use JSON, TOML or RON directly. You can use the same
state declaration with each engine, subject to the format's supported values.
Unsupported values produce an error.

**Value rules.** A field's `rule` can normalize values read from storage or
written by your code: clamp a size, for example, or replace an unavailable
theme. Corrections are written back to the store. Checks involving several
fields belong in a custom `Open` implementation.

**Schema changes.** The store records each struct's schema. When you increase
its version and open with `migrate`, the library runs your migration steps.
Changing the schema without increasing the version produces a drift report
with a diff, but does not block startup.

**External edits.** Text engines watch their files and notify subscribers when
another program changes a value. Before saving, they merge pending writes into
the current document rather than replacing it with an older copy.

## Two kinds of state

Choose whether your application needs reactive handles or plain values.
Both modes use the same storage, schemas and migrations.

- **Reactive**, the default: read, write and subscribe through field handles.
  Most examples in the book use this mode.
- **Persistent-only**: load ordinary struct fields and save them explicitly.
  Use it when the application manages updates itself and does not need
  subscriptions. Loaded structs do not receive changes made elsewhere.

## What it is for, and what it is not

Use it for window layouts, open panels, filters, recent-file lists and caches
that should survive a restart. The API supports both small settings structs
and collections with thousands of keys.

It is not a general-purpose database: there is no query language, no indexes
and no public transaction API. Data is addressed by path. You can open several
stores, but opening the same file more than once is engine-dependent; see
[Opening a store](/amethystate/store/opening/).

## Where to go next

[Quick start](/amethystate/getting-started/quick-start/) is the shortest path
from nothing to a running store.
