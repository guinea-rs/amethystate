---
title: Installation
sidebar:
  order: 2
---

```toml
[dependencies]
amethystate = { version = "0.28", features = ["redb"] }
```

Enable a storage engine through a Cargo feature; none is enabled by default.
Choose it in the application crate. Libraries that only declare state structs
or provide framework adapters can leave the engine choice to the application.

## Choosing an engine

Each store uses one engine. Cargo features select which engines are available
in your build; `StoreBuilder::backend` selects one when opening a store.
If you omit it, the builder uses the first available engine in the order below.

| feature | engine | where it lives |
| --- | --- | --- |
| `redb` | redb | a `.redb` file |
| `sqlite` | SQLite | a `.db` file |
| `json` | JSON | a `.json` file |
| `toml` | TOML | a `.toml` file |
| `ron` | RON | a `.ron` file |
| `localstorage` | the browser's localStorage | keys under `amethystate.<store name>.` in the page |

**Text engines write two files**: the data document and a `.meta` sidecar with
schema versions, field descriptions and migration progress. The document is
readable on its own, but without the sidecar the store loses its schema history.
Back up and move both files together. redb and SQLite keep this metadata inside
the database file.

Formats support different values. The
[engine comparisons](/amethystate/choosing/absent-or-null/) show the results
of writing the same values with each engine.

A build with no engine in it still compiles, and a store opened in it is
refused: `StoreBuilder::build` answers `OpenStore::WouldNotOpen`, naming the
features to turn on.

The `memory` feature adds an in-memory engine. It is never selected by default:
choose it explicitly or use it as a fallback when a file cannot be opened.
See [Opening a store](/amethystate/store/opening/#or-running-in-memory).

### In a browser

For a browser application targeting `wasm32-unknown-unknown`, enable
`localstorage`. It stores one JSON value per key in the browser's localStorage,
which you can inspect in developer tools. Writes are synchronous, without a
flush delay, and quota exhaustion is reported as a write error. Changes from
other tabs are delivered as external updates. This engine can only open in a
browser page.

```toml
amethystate = { version = "0.28", features = ["localstorage"] }
```

### Several engines at once

Cargo features are additive. Without an explicit choice, the builder uses the
first enabled engine in this order: **redb, SQLite, JSON, TOML, RON, localStorage**.
A dependency enabling `redb` can therefore change which engine your store uses.

You can enable several engines, for example in a conversion tool or a test
suite. Select one explicitly with `StoreBuilder::backend` so dependency features
do not affect the file format.

### SQLite

SQLite is built from source, so you need a C toolchain. In exchange the minimum
SQLite version is this library's choice rather than whatever the user's
distribution ships.

Bundling matters because SQLite features have minimum version requirements.
For example, `STRICT` tables require SQLite 3.37. An older SQLite can report
an unsupported schema as corruption even when the file is intact.

```toml
amethystate = { version = "0.28", features = ["sqlite"] }
```

## Migrating from an existing solution

See [Migrating from a custom solution](/amethystate/migrations/custom/).

## guinea

A guinea timer whose period follows a stored setting: see
[guinea](/amethystate/integrations/guinea/).
