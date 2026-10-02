---
title: Declaring a struct
sidebar:
  order: 1
---

The `#[amethystate]` macro transforms a plain Rust struct into a persistent
state container. This page is about the struct itself: where it sits, which
"line" of versions it belongs to, and how structs hold one another. The rest of
this section is one subject a page:

- [Fields](/amethystate/state/fields/) - where each field is stored and how;
- [When a value will not read](/amethystate/state/unreadable-values/) - bytes
  that do not decode, and a key deleted under a field;
- [Rules](/amethystate/state/rules/) - what a value may be, judged both ways:
  on every way in and every way out;
- [Opening a struct your own way](/amethystate/state/opening-structs/) -
  `open = manual`, and invariants between fields;
- [What serde says here](/amethystate/state/serde/).

## Struct attributes

```rust
#[amethystate(prefix = "network", version = 1, mode = "reactive")]
pub struct NetworkState { ... }
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `prefix` | `String` | The place in the store these fields hang under. A root struct needs this or `as_root`. |
| `version` | `u32` | Schema version for migrations. Defaults to `0`. |
| `id` | `String` | Which line of versions at the prefix this struct belongs to. Needed only where structs share a prefix; see below. |
| `rename_all` | `&str` | How every field's own name is spelled where it is stored. A closed list of eight: `lowercase`, `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case`, `SCREAMING-KEBAB-CASE`. Anything else is a compile error. A field with a `path` of its own is not touched by it. |
| `mode` | `String` | Code generation mode: `"reactive"` (default) or `"persistent"`. `"reactive"` opens with `new_with` and makes every field a `Field`; `"persistent"` is a plain struct that `load` reads and `save` writes, see [Persistent-only mode](/amethystate/getting-started/quick-start/#persistent-only-mode), and needs a `prefix` or `as_root`. |
|`as_root`| `flag` | Fields sit at the top of the store, with no name above them. Written **instead of** `prefix` — the two say different things about the same place, so writing both is a compile error. |
| `on_unreadable` | variant | What opening does about a stored value that will not decode. `Refuse` (the default) or `UseDefault`. See [When a value will not read](/amethystate/state/unreadable-values/). |
| `on_delete` | variant | What a field does when its key is deleted under it. `Keep` (the default) or `UseDefault`. |
| `unreadable_entries` | variant | What a `ReactiveMap` does with an entry it cannot read. `Refuse` (the default) or `Skip`. Fields that are not maps have no entries and ignore it. |
| `open` | `manual` | The struct's `Open` is written by hand, and nothing opens it over the global store. See [Opening a struct your own way](/amethystate/state/opening-structs/). |

A struct takes no `rule`: a rule judges one value, and goes on a field. An
invariant between fields goes in the struct's own `Open`.

A struct that says neither `prefix` nor `as_root` is a component: it has no
place of its own, and it goes inside another one through `nested`. Its place is
the field that holds it.

## Places and lines

A `prefix` claims nothing by itself - the fields under it do, each its own path.
So two structs over the same prefix live together happily until two of their
paths meet, and the second one to open over a taken path is refused. A map is
the exception and owns everything beneath it, because its keys are made while
the program runs. That is a whole subject of its own, and the one that decides
how a `prefix` and a dotted stored name interact:
[Who owns which place](/amethystate/concepts/claims/).

Places are one matter and versions another. Versions are counted per line, and
a line is the prefix together with the struct's `id`. Structs written without
an `id` are versions of the prefix's one unnamed line, so where several structs
share a prefix, all but one of them need an `id`:

```rust
#[amethystate(prefix = "ui", id = "panels", version = 3)]
pub struct Panels { ... }
```

Two unnamed structs at one prefix and one version are one version declared
twice, and the store does not open: `MigrationError::DeclaredTwice` names both.
Lines that share a prefix still own their places apart, and two that own one
place - or one inside the other's - do not open either:
`MigrationError::ClaimedTwice` names the place and both structs.
An `id` is kept once written. A struct given another one starts a new line, and
what the old line recorded comes back as drift.

## Nested structs

Structs without a `prefix` are components — they have no storage namespace of their own and are embedded into a parent struct via `nested`. The parent's prefix is prepended to all nested fields.

```rust
#[amethystate]
pub struct DatabaseConfig {
    #[amestate(default = "localhost".to_string())]
    pub host: String,
}

#[amethystate(prefix = "sys")]
pub struct SystemSettings {
    #[amestate(nested)]
    pub db: DatabaseConfig, // stored as "sys.db.host"
}
```

A nested field can also put its fields at its holder's level, with no segment
of its own: [`flatten`](/amethystate/state/fields/#a-field-whose-paths-sit-at-its-holders-level).

## Sharing one place between two structs

Two structs cannot both declare the same place - the second to open is refused.
Where one value has to be reachable from two sides, address it by path from the
one that did not declare it: [Kv](/amethystate/primitives/kv/) reads and writes
anywhere no struct has claimed, and
[Who owns which place](/amethystate/concepts/claims/) is what decides where that
line falls.

## Root-level storage (`as_root`)

By default, all fields are stored under the struct's `prefix`. With `as_root`, fields are written directly to the store root with no namespace.

```rust
#[amethystate(mode = "persistent", as_root)]
pub struct AppConfig {
    #[amestate(default = "acme".to_string())]
    pub name: String,

    #[amestate(default = false)]
    pub verbose: bool,
}
```

This produces a file like:

```toml
name = "acme"
verbose = false
```

That is the shape to ask for when the file is read by something other than this
crate — a config somebody edits by hand, or one whose keys another program
already expects at the top level. Root fields are claimed like any others, so
two structs reaching for the same key still collide:
[Who owns which place](/amethystate/concepts/claims/).
