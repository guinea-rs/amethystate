---
title: Manual Migrations
sidebar:
  order: 22
---

Use manual steps when you need direct key access, data from another prefix,
or explicit cleanup of map entries. They run in the same migration engine
as steps declared with `#[migrate]`.

## Entry point

Register manual steps with the builder's `.migrations()` method:

<!-- shown: registering steps by hand -->
```rust
let (store, report) = StoreBuilder::new(app)
    .migrations(|m| {
        m.for_prefix("net")
            .step(1, "move off the privileged port", |ctx| {
                ctx.set("port", &8080u16)
            });
    })
    .migrate()?;
```
<!-- /shown -->

Finish with `migrate` to run both these steps and linked `#[migrate]` functions
and receive a report. After `.migrations()`, `build` is unavailable, preventing
registered steps from being silently ignored. Set the other store options first.

Steps within a line run in version order. Cross-prefix dependencies are resolved
when a step reads them; see [Reading across prefixes](#reading-across-prefixes).

## Defining a step

```rust
m.for_node::<Profile>()
    .step(2, "split full name", |ctx| {
        let full_name = ctx.get::<String>("full_name")?.unwrap_or_default();
        let mut parts = full_name.splitn(2, ' ');
        ctx.set("first_name", &parts.next().unwrap_or_default().to_string())?;
        ctx.set("last_name", &parts.next().unwrap_or_default().to_string())?;
        ctx.delete("full_name")?;
        Ok(())
    });
```

`for_node::<T>()` selects the line from the struct's prefix and `id`.
`for_prefix("net")` selects the unnamed line at that prefix;
`for_named("ui", "panels")` selects its named line.

`.step(version, description, closure)` converts `version - 1` to `version`.
Only one step may target each version in a line. A manual step that duplicates
a generated one is rejected with `MigrationError::StepTwice` before that line
runs.

A generated step cannot change its prefix or `id`; the macro rejects that
transition. To move data between locations, use explicit global reads and writes.

## The context API

A step's `ctx` reads and writes stored keys directly. Unless marked global,
paths are relative to the line's prefix.

### Basic operations

| Method | Description |
| --- | --- |
| `ctx.get::<T>(key)` | Read a value as `Result<Option<T>>`. |
| `ctx.set(key, value)` | Write a value. |
| `ctx.delete(key)` | Delete a key. |
| `ctx.rename(from, to)` | Copy to the new path and delete the source; do nothing if the source is absent. |

### Compound operations

These helpers combine reading, conversion and writing:

| Method | Description |
| --- | --- |
| `ctx.transform::<TOld, TNew>(key, f)` | Replace an existing value with `f(value)`; do nothing if absent. |
| `ctx.merge::<T1, T2, TNew>((k1, k2), into, f)` | Combine two values at a destination and delete the sources; do nothing if either is absent. |
| `ctx.split::<TOld, T1, T2>(from, (k1, k2), f)` | Split one value into two destinations and delete the source; do nothing if absent. |

For example:

```rust
// multiply a value in place
ctx.transform::<u16, u16>("sample_rate", |r| Ok(r.saturating_mul(10)))?;

// combine first_name + last_name into display_name, remove originals
ctx.merge::<String, String, String>(
    ("first_name", "last_name"),
    "display_name",
    |f, l| Ok(format!("{f} {l}")),
)?;

// split "host:port" into two separate keys
ctx.split::<String, String, u16>(
    "address",
    ("host", "port"),
    |s| {
        let (h, p) = s.split_once(':').unwrap();
        Ok((h.to_string(), p.parse()?))
    },
)?;
```

### Collections

| Method | Description |
| --- | --- |
| `ctx.scan_map::<K, V>(key)` | Read entries beneath `prefix.key` as an `IndexMap<K, V>`, ordered like the map. |

Use this to migrate a `ReactiveMap` without constructing `AmeData`:

```rust
let old_routes = ctx.scan_map::<String, String>("routes")?;
ctx.delete_prefix("routes")?;

for (name, url) in &old_routes {
    ctx.set(&format!("endpoints.{name}"), url)?;
}
```

`delete_prefix` removes the old map subtree in one call.

`scan_map` must decode every key and value. An unreadable entry fails the step
and rolls back its transaction, rather than silently losing that entry during
conversion.

### Global access

| Method | Description |
| --- | --- |
| `ctx.global_get::<T>(path)` | Read a full path anywhere in the store. |
| `ctx.global_set(path, value)` | Write a full path anywhere in the store. |

These methods do not prepend the current line's prefix. Use them when reading
or moving data outside it:

```rust
let plan = ctx.global_get::<String>("identity.plan")?.unwrap();
```

### Scoping

`ctx.scoped(sub_prefix)` creates a context beneath the current prefix.
It is also used internally by `ctx.nested()`.

## Values from the application

A `#[migrate]` function cannot capture local variables, and registered closures
must be `'static`. Supply dependencies such as lookup tables through the
builder instead. They are stored one per type and retrieved by that type:

<!-- shown: a value the application provides -->
```rust
struct LegacyDefaults {
    port: u16,
}

let (store, report) = StoreBuilder::new(app)
    .provide(LegacyDefaults { port: 8080 })
    .migrations(|m| {
        m.for_prefix("net")
            .step(1, "fill in the port the old build assumed", |ctx| {
                let port = ctx.require::<LegacyDefaults>()?.port;
                ctx.set("port", &port)
            });
    })
    .migrate()?;
```
<!-- /shown -->

`ctx.provided::<T>()` returns `None` if the dependency is absent.
`ctx.require::<T>()` instead returns `RunStep::NothingProvided`, identifying
the requested type and the available types.

## Reading across prefixes

When a step calls `ctx.global_get` on another prefix, the engine migrates that
prefix first, inside the same transaction. No separate ordering list is needed:

```rust
m.for_node::<Profile>()
    .step(2, "snapshot plan from identity", |ctx| {
        let plan = ctx
            .global_get::<String>("complex_identity.plan")?
            .expect("reading it is what migrates it");
        ctx.set("plan_snapshot", &plan)?;
        Ok(())
    });
```

The read therefore returns migrated data. If `Workspace` reads `Profile` and
`Profile` reads `Identity`, the dependency reads complete `Identity` before
`Profile`, then resume `Workspace`, regardless of registration order.

Reading a prefix whose migration is already active creates a dependency cycle.
The error reports the complete cycle so you can remove the circular dependency.

## Reaching the context from #[migrate]

A `#[migrate]` function can also accept a `MigrationContext` as its second
argument. All methods above are available there. See
[Defining steps](/amethystate/migrations/defining-steps/#cleanup-the-step-already-does)
for generated cleanup and cases needing explicit deletion.

## Failure and rollback

The engine groups a migration and the prefixes it reaches into one transactional
**component**. A failed step rolls back that component, not unrelated components.
Other components may already have committed or may still migrate successfully.

The store then refuses to open with `OpenStore::Migrating`, which contains the
complete report. This avoids returning a store whose data does not match the
requested schema:

<!-- shown: a step that fails refuses the open -->
```rust
let refused = StoreBuilder::new(app)
    .migrations(|m| {
        m.for_prefix("net").step(1, "turns the data down", |_| {
            Err(MigrationError::Custom("this data is not ours".into()).into())
        });
    })
    .migrate();

let Err(OpenStore::Migrating {
    report: Some(report),
    ..
}) = refused
else {
    panic!("a failed step let the store open");
};
```
<!-- /shown -->

The report identifies failed prefixes and their reasons. Those components
retain their previous versions; successful components have advanced.

For an application that can run without persisted state, `or_in_memory()`
before `migrate` provides a fresh in-memory fallback. `build` skips migration
steps, but is not a repair operation: use it only when your code explicitly
supports the stored layout.

<!-- shown: reading what failed, then opening without the steps -->
```rust
for component in &report.components {
    if let ComponentOutcome::Failed { error, .. } = &component.outcome {
        eprintln!("{:?} was left as it was: {error:?}", component.prefixes);
    }
}

let store = StoreBuilder::new(app).build()?;
```
<!-- /shown -->

A failed component lists the prefixes involved. Step errors include the prefix,
target version and store file.

### What a stopped open leaves on disk

A step error rolls back its component before commit. A process interruption
while committing text files is different: data and metadata are separate files
and cannot be replaced together.

Text engines first write metadata recording the pending commit, then data,
then finalized metadata. On the next open:

- if the data matches the pending new state, the metadata commit is completed;
- if it matches the old state, the pending record is cleared and migration can
  run again;
- if it matches neither, opening fails rather than guessing which schema applies.
