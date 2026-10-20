---
title: Declaring a struct
sidebar:
  order: 1
---

`#[amethystate]` generates stored state from a Rust struct. This page covers
its storage location, version identity and nested components.

Related pages cover:

- [Field attributes](/amethystate/state/fields/) — paths, defaults and encoding;
- [Unreadable values](/amethystate/state/unreadable-values/) — read failures and deleted keys;
- [Rules](/amethystate/state/rules/) — normalizing values;
- [Custom construction](/amethystate/state/opening-structs/) — `open = manual` and cross-field checks;
- [Serde](/amethystate/state/serde/) — attributes on declarations versus leaf values.

## Struct attributes

```rust
#[amethystate(prefix = "network", version = 1, mode = "reactive")]
pub struct NetworkState { ... }
```

| Attribute | Type | Description |
| --- | --- | --- |
| `prefix` | `String` | Base path for the fields. A standalone struct needs `prefix` or `as_root`. |
| `version` | `u32` | Migration schema version; defaults to `0`. |
| `id` | `String` | Distinguishes independent version histories sharing a prefix. |
| `rename_all` | `&str` | Stored field-name convention: `lowercase`, `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case` or `SCREAMING-KEBAB-CASE`. Does not change explicit `path` attributes. Other values are compile errors. |
| `mode` | `String` | `"reactive"` by default: access values through handles. `"persistent"`: load and save ordinary fields explicitly. See [Persistent-only mode](/amethystate/getting-started/quick-start/#persistent-only-mode). |
| `as_root` | flag | Store fields directly at the root. Cannot be combined with `prefix`. |
| `on_unreadable` | variant | Read-failure policy: `Refuse` by default, or `UseDefault`. |
| `on_delete` | variant | Policy for a field's deleted key: `Keep` by default, or `UseDefault`. |
| `unreadable_entries` | variant | Map-entry read policy: `Refuse` by default, or `Skip`. Applies only to maps. |
| `open` | `manual` | Implement `Open` yourself; global-store constructors are not generated. |

A `rule` belongs to a field, not a struct. Use a custom `Open` implementation
for checks involving several fields.

Without `prefix` or `as_root`, the struct is a component embedded through
`nested`; its parent determines its location.

## Places and lines

A struct reserves its field paths, not the entire `prefix`. Two structs can
share a prefix if their field paths do not overlap. A map reserves its whole
subtree because its entry keys are not known in advance.
[Ownership](/amethystate/concepts/claims/) explains these rules.

Version histories are separate from path ownership. A **line** is identified
by its prefix and optional `id`. Structs without an `id` belong to that prefix's
unnamed line. When independent structs share a prefix, give all but at most one
of them an explicit `id`:

```rust
#[amethystate(prefix = "ui", id = "panels", version = 3)]
pub struct Panels { ... }
```

Two declarations for the same line and version produce
`MigrationError::DeclaredTwice`. Different lines with overlapping owned paths
produce `MigrationError::ClaimedTwice`; an `id` does not make overlapping
storage safe.

Keep a released `id` stable. Changing it starts another history and leaves the
previous line recorded as drift.

## Nested structs

A component without its own prefix is embedded in a parent through `nested`.
Its fields are stored beneath the parent's prefix and the nested field's name:

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

Use [`flatten`](/amethystate/state/fields/#a-field-whose-paths-sit-at-its-holders-level)
to omit the nested field's segment from storage paths.

## Sharing one place between two structs

Do not declare the same stored value twice. Instead, share clones of the
existing field handle with the components that need it, or pass a
`ReactiveCell` made from that field.

`Kv` is intended for unclaimed paths; it does not bypass declared ownership.
See [Ownership](/amethystate/concepts/claims/).

## Root-level storage (`as_root`)

With `as_root`, fields are stored directly at the root rather than beneath
a prefix:

```rust
#[amethystate(mode = "persistent", as_root)]
pub struct AppConfig {
    #[amestate(default = "acme".to_string())]
    pub name: String,

    #[amestate(default = false)]
    pub verbose: bool,
}
```

For example, a TOML file contains:

```toml
name = "acme"
verbose = false
```

Use this layout when an external program expects top-level keys or when it
makes a hand-edited config easier to read. Root fields still reserve their
paths, so two structs cannot declare the same key. See
[Ownership](/amethystate/concepts/claims/).
