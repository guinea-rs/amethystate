---
title: Migrations
sidebar:
  order: 20
---

## What migrations are for

Suppose an older release stored `host`, and the next release expects `address`.
Changing the Rust field name does not move the saved value. A migration reads
the old data and writes the representation expected by the new release.

You assign versions to state structs and write the conversion functions.
`StoreBuilder::migrate` runs the required steps while opening the store,
before returning it to your application.

## What migrates and what doesn't

The migrator works exclusively with persistent data.

| Field kind | Migrates? |
|------------|-----------|
| Regular fields | ✓ |
| `nested` structs | ✓ |
| `volatile` fields | ✗ — in-memory only, no stored data |

## How versioning works

Every `#[amethystate]` struct has a `version` attribute that defaults to `0`:

```rust
#[amethystate(prefix = "app", version = 2)]
pub struct AppConfig { ... }
```

Versions are tracked per **line**: a prefix together with the struct's `id`,
if it has one. The migrator compares the stored version with the version in code
and follows the registered steps between them.

If the chain has a gap, the store fails to open. Opening data written by a newer
schema is also refused: an older application must not reinterpret it as old data.

## Automatic collection

`migrate` collects every step declared with `#[migrate]` and hands back a report of what it did:

```rust
let (store, report) = StoreBuilder::new("./app.redb").migrate()?;
```

Steps declared with the macro need no separate registration. You must still
call `migrate` to run them: `build` does not execute migration steps, even when
the binary contains `#[migrate]` functions.

For steps that need lower-level access or data from another struct, see
[Manual Migrations](/amethystate/migrations/manual/).

## What happens on a clean install

A new, empty store starts at the current schema version without running old
migration steps. Fields use their declared defaults when you open the state
struct; there is no old user data to convert.

## When the struct changed and the version did not

Changing the schema without increasing `version` does not request a migration.
The library reports the difference as drift and continues opening the store.
The report repeats on later opens until you resolve the mismatch; see
[Schema drift](/amethystate/migrations/drift/).
