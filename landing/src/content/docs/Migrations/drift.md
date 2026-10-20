---
title: Schema drift
sidebar:
  order: 23
---

Schema drift means stored declarations no longer match the code, despite
unchanged version numbers. For example, a field may have moved or a scalar
may have become a map without a migration step.

## What is compared

When stored and declared versions match, the engine compares their owned
storage paths. A **place** is a stored leaf or map level, not a nested wrapper.
Adding or removing a wrapper segment therefore appears as moves of its children:

```text
  declared                    owned
  ------------------------------------------------
  ui            nested        -           a nested struct is not a place
    theme       field         ui.theme

  ui            flattened     -           and lends no segment either
    theme       field         theme
```

The comparison uses paths, not Rust field names. A nested `ui.theme` field and
`path = "ui.theme"` can describe the same location. Conversely, identical Rust
field names under different prefixes are different locations.

## The verdicts

| Change | Classification | Meaning |
| --- | --- | --- |
| A previously declared path was removed | **breaking** | Old data is no longer covered by the declaration. |
| A leaf became a map level, or the reverse | **breaking** | The stored layouts differ. |
| A new path was declared | **depends on stored data** | An empty path is safe; existing undeclared data needs review. |
| Optionality changed | **harmless** | The path is still owned by the declaration. |

A breaking difference produces a drift report. Related differences are included
for context, even if they are harmless individually.

A rename alone looks like a removed path plus an added path. Use
[`#[rename(old => new)]`](/amethystate/migrations/defining-steps/) on a migration
to record the intended relationship.

## It does not block startup, and it does not go away

A drift report by itself does not block opening. The old recorded schema is
retained, so the same mismatch is reported on subsequent opens until resolved.

If the change was intended, increase the version and write a migration.
If not, restore the old layout. Development-only stores have another option:

## While the shape is still being worked out

During early development, you may prefer discarding obsolete data to maintaining
migration steps for every experimental change:

<!-- shown: letting go of what nothing declares -->
```rust
let (store, report) = StoreBuilder::new(path)
    .rules(|r| r.on_undeclared(OnUndeclared::Drop))
    .migrate()?;

assert!(!report.has_drift());
```
<!-- /shown -->

`OnUndeclared::Drop` deletes keys beneath declared prefixes that no current
struct declares, then records the current schema. A renamed field starts from
its default; its old value is **deleted, not moved**.

Cleanup runs after the migration steps and is skipped if a migration failed.
Keys outside declared prefixes are untouched. The root is never swept:
`as_root` declares particular fields, not ownership of all root data.

Use this policy only for disposable development data. The default
`OnUndeclared::Keep` preserves undeclared data and reports drift.

## Reading it

`MigrationReport::has_drift` indicates whether drift was found.
`component.nagging` contains a `NaggingRecord` per line, including its prefix,
`id`, changed paths, classifications and field-name diff:

```rust
let (store, report) = StoreBuilder::new("./app.redb").migrate()?;

for record in report.components.iter().flat_map(|c| &c.nagging) {
    for moved in &record.moved {
        println!("{:?} {moved}", moved.verdict());
    }
}
```

Paths in each record are relative to its prefix: `token`, not `app.token`.
The record also contains that prefix.

## Rendering it

Both `build` and `migrate` log drift through `tracing`. Enable `diagnostics`
for compiler-style warnings instead of individual log lines:

```toml
amethystate = { version = "0.28", features = ["diagnostics"] }
```

For example, the release that wrote the data declared:

```rust
#[amethystate(prefix = "app", version = 2)]
pub struct Settings {
    #[amestate(default = None)]
    pub host: Option<String>,

    #[amestate(default = {})]
    pub port: ReactiveMap<String, u16>,

    #[amestate(default = String::new())]
    pub token: String,
}
```

The new release declares the same version with a different layout:

```rust
#[amethystate(prefix = "app", version = 2)]
pub struct Settings {
    #[amestate(default = "localhost".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```

`token` was removed, `port` is no longer a map, and `host` is no longer optional.
The version remains `2`:

```
amethystate::drift

  ⚠ `app` holds a shape this build does not declare
  help: raise the struct's `version` and write a step for it, or declare the
        released places again

Warning:
  ⚠ `app.token` was declared before and is not now
  help: what an earlier build wrote there is under no declaration now

Warning:
  ⚠ `app.port` was written as Map and is declared as Field
  help: one value and a level of entries are not the same thing on disk

Advice:
  ☞ `app.host` may no longer hold nothing
  help: what was written there as nothing reads back as the declared default
```

Diagnostics put breaking changes first, then changes needing review, then
other differences. Raw records retain comparison order.

`diagnostics` is disabled by default and changes presentation only. Drift
checking and `has_drift` work without it. Use `MigrationReport::drift` to obtain
the diagnostics for your own renderer.

## What it does not compare

Drift checking does not inspect leaf types or their serialized contents.
Changing `u16` to `u32` keeps the same owned path and is not itself drift.
An incompatible value is reported when the field is read, through
`on_unreadable` and `disagreements()`. See [Errors](/amethystate/concepts/errors/).

## What has to be running for any of this

Drift checking runs on every open, through `build` or
[`migrate`](/amethystate/store/opening/), even without registered migration steps.

Only `migrate` executes the steps. Declaring `#[migrate]` functions but calling
`build` does not apply them; a drift warning is not a substitute for migration.
