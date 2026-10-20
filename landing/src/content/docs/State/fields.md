---
title: Fields
sidebar:
  order: 2
---

Without `#[amestate]`, a field uses its Rust name as its storage key and
`Default::default()` as its initial value. Add attributes to change these defaults:

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    pub counter: u32, // no annotation — uses Default::default(), stored as "app.counter"

    #[amestate(default = 8080)]
    pub port: u16,
}
```

| Attribute | Type | Description |
| --- | --- | --- |
| `path` | `&str` | Stored path relative to the struct's prefix. Dots separate levels. |
| `default` | expression | Value used when initializing an empty path; otherwise `Default::default()`. |
| `nested` | flag | Embed another `#[amethystate]` struct. |
| `flatten` | flag | Omit a nested field's own path segment. |
| `volatile` | flag | Keep only in memory, initialized from the default on each run. |
| `with`, `serialize_with`, `deserialize_with` | path | Custom serde-compatible encoding and decoding functions. |
| `on_unreadable` | variant | Override the struct's read-failure policy. Not valid on maps; use `unreadable_entries`. |
| `on_delete` | variant | Policy after deletion. For a whole map level, `UseDefault` restores declared defaults; `Keep` leaves it empty. Deleting an individual entry removes it under either policy. |
| `unreadable_entries` | variant | Map-only: `Skip` omits unreadable entries; `Refuse` (default) fails with the entry path. |
| `rule` | `fn` | Normalize reads and writes. Defaults are not processed. See [Rules](/amethystate/state/rules/). |

Combine attributes in one `#[amestate(..)]` or use several annotations:

```rust
#[amestate(path = "panels.left.visible")]
#[amestate(default = true, on_delete = Keep)]
pub left_panel_visible: bool,
```

Repeating an attribute is a compile error rather than an implicit override.

## Where a field goes

`path` sets a field's stored path. The struct's `rename_all` applies a naming
convention to fields without an explicit path. Dots in `path` create levels,
not literal dots in a key:

<!-- shown: a struct that says where its fields go -->
```rust
#[amethystate(prefix = "net", rename_all = "camelCase")]
pub struct NetState {
    #[amestate(default = 8080u16)]
    pub listen_port: u16,

    #[amestate(path = "tls.enabled", default = false)]
    pub tls: bool,
}
```
<!-- /shown -->

The resulting paths are `net.listenPort` and `net.tls.enabled`.

## A field whose paths sit at its holder's level

Use `flatten` on a `nested` field to store its children directly at the parent
level, without a segment for that field:

<!-- shown: a nested struct whose fields sit at their holder's level -->
```rust
#[amethystate(prefix = "editor")]
pub struct Editor {
    #[amestate(nested, flatten)]
    pub window: Window,
}
```
<!-- /shown -->

The path becomes `editor.width`, not `editor.window.width`.

Overlapping paths from flattened children or adjacent fields are compile errors.
This includes ancestor conflicts: a flattened `window` subtree conflicts with
an adjacent `path = "window.width"`, even though their complete paths differ.

**Changing a released `path` or adding/removing `flatten` requires a migration.**
Otherwise the old data remains at the old path while the new code reads a
missing path and uses its default.

## A field stored some other way

To customize a field's stored representation, provide encoding and decoding
functions:

```rust
#[amestate(with = since_the_epoch)]
pub opened: SystemTime,
```

`with = m` uses `m::serialize` and `m::deserialize`. You can override one
function with `serialize_with` or `deserialize_with`; the other then uses the
type's normal serde implementation. Make sure the two representations match.

The field type must still implement `Serialize` and `Deserialize`.
The custom functions change its stored representation, not those trait bounds.
Wrap a foreign type without serde support in a newtype that implements them.

Unknown `amestate` options produce compile errors listing accepted attributes.

## Attributes that are not this macro's

Other field attributes are forwarded to the generated field and getter.
Documentation comments, `#[allow]` and `#[deprecated]` retain their usual meaning.
A serde helper attribute without a serde derive is rejected by rustc.

Field-level `#[cfg]` is not supported: a field must remain consistent across
the generated struct, constructors, snapshots and schema metadata. Apply `cfg`
to the whole struct, or keep the field and select its value at runtime.

See [Serde](/amethystate/state/serde/) for the distinction between state
attributes and attributes on serialized values.

## Volatile fields

Volatile fields stay in memory and start from their default on each run.
Use them for transient UI state:

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    #[amestate(default = 8080)]
    pub port: u16,

    #[amestate(default = false, volatile)]
    pub loading: bool, // always starts as false, never written to disk
}
```

A volatile field can still have a [rule](/amethystate/state/rules/).
It normalizes writes even though the value is not persisted.
