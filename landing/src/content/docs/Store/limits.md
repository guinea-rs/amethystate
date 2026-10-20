---
title: Value and path limits
sidebar:
  order: 14
---

The store rejects values that its codec cannot read back. You can add stricter
limits, but cannot disable these safety checks.

## How deeply a value may nest

Depth includes nested structs, sequences and maps inside a value. Recursive
types such as trees or menus can have different depths for different inputs.

A serializer may accept deeper data than its deserializer can read. Without a
write-time check, saving such a value could make the next open fail. In a text
engine, one excessively deep value can prevent parsing the whole document.

The store therefore counts nesting during serialization and stops before the
reader's limit. Counting in the codec pass also avoids first building a deeply
nested intermediate value that could overflow the stack.

<!-- shown: how deep the engine you are running reads -->
```rust
let engine = default_backend();
println!("{}: {} levels", engine.extension(), engine.depth_ceiling());
```
<!-- /shown -->

The enforced limits are covered by `tests/probe_*.rs`:

| engine | levels |
| --- | --- |
| redb | 512 |
| JSON | 127 |
| SQLite | 127 |
| TOML | 80 |
| RON | 64 |

The redb codec has no useful built-in recursion limit. Its 512-level ceiling is
a library safety limit intended to stop before stack exhaustion, rather than a
measured promise about every platform's stack size.

### The path spends from the same budget

Paths consume depth budget too. In text documents, `ui.panels.left` adds three
levels before the value's own nesting begins. The shared check also subtracts
path depth for SQLite even though its key does not add JSON nesting; that is a
conservative limit.

A depth error reports the total budget, the levels used by the path and the
remaining allowance for the value.

## The two caps you can set

Configure stricter limits through `limits`. Neither option can raise the
engine's safety ceiling:

<!-- shown: capping how deep a path may go -->
```rust
let store = StoreBuilder::new(settings)
    .limits(|l| l.key_depth(4))
    .build()?;

let deep = StorePath::from_segments(["a", "b", "c", "d", "e"]);

if let Err(refused) = store.set(&deep, &1u32) {
    println!("{refused:?}");
}
```
<!-- /shown -->

`key_depth` rejects a path with too many segments before encoding its value.
This is useful for runtime-generated paths, where accidental recursive nesting
should fail at the write rather than leave unreadable data.

<!-- shown: promising the contents stay readable on another engine -->
```rust
let store = StoreBuilder::new(settings)
    .limits(|l| l.portable_across([default_backend()]))
    .build()?;
```
<!-- /shown -->

`portable_across` lists other engines that must also be able to read the values.
The effective depth limit is the lowest among the active and listed engines.
For example, redb plus RON portability allows 64 levels rather than 512.

Portability checks also cover these value properties:

| property | limitation |
| --- | --- |
| `NaN` and infinities | JSON and SQLite cannot preserve them as numeric values. |
| enum variants | The RON value tree used by the engine cannot retain them. Types such as `IpAddr` that serialize as strings in a human-readable format are checked in that representation. |
| `Some(None)` | All engines except RON collapse nested `Option` distinctions. |
| `u128` and `i128` | TOML and RON do not support the 128-bit integer type, even for small values. |
| integers outside `i64` | Outside TOML's integer range. |
| integers outside both `i64` and `u64` | JSON's value representation cannot retain them as integers. |

The active engine is always included. A JSON store therefore rejects a NaN even
without an explicit portability setting.

Name the engines you actually need. An implicit “all engines” policy would
change its meaning whenever another engine was added.

## What this does not cover

These checks cover depth and the properties listed above, not every difference
between formats. See [Choosing an engine](/amethystate/choosing/absent-or-null/)
for other tested limitations.
