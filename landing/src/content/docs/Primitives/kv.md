---
title: "Kv"
sidebar:
  order: 11
---

`Kv` provides path-based access without a state struct. Use it for keys chosen
at runtime or for data that does not need a declared schema.

<!-- shown: reading and writing without a schema -->
```rust
let kv = store.kv();

kv.set("theme", &"dark".to_string())?;
let theme = kv.get::<String>("theme")?;

let ui = kv.namespace("ui");
ui.set("width", &800u32)?;
let under_ui = ui.keys()?;

kv.remove("theme")?;
```
<!-- /shown -->

## A name is one level

Each name passed to `Kv` is one path segment. `kv.set("ui.width", ..)` writes
a single key named `ui.width`. To write `width` inside `ui`, use
`kv.namespace("ui").set("width", ..)`.

A literal dot in a name is escaped in storage, so these calls address different
keys. Do not use a dotted name as a substitute for `namespace`.

`keys()` returns sorted paths at every depth beneath the current namespace.
Use a narrower namespace to restrict the listing; the root handle lists the
whole store.

<!-- shown: what a listing covers -->
```rust
kv.set("theme", &"dark".to_string())?;
kv.namespace("ui").set("width", &800u32)?;
kv.namespace("ui").namespace("panel").set("left", &true)?;

let from_ui: Vec<String> = kv
    .namespace("ui")
    .keys()?
    .iter()
    .map(|key| key.to_string())
    .collect();

assert_eq!(from_ui, ["ui.panel.left", "ui.width"]);
```
<!-- /shown -->

Returned paths include their namespace: `width` inside `ui` appears as
`ui.width`. A listing from `ui` does not include a root-level `theme` key.

## Cells and maps

<!-- shown: a cell and a map with nothing declared -->
```rust
let width = kv.namespace("ui").cell("width", 800u32)?;
let flags = kv.map::<String, bool>("flags")?;

flags.insert("dark".to_string(), &true)?;
```
<!-- /shown -->

These methods return a [`ReactiveCell`](/amethystate/primitives/reactive-cell/)
or `ReactiveMap`, with the same subscriptions as declared fields. A cell's type
is inferred from its default value.

## Where a struct already lives

`Kv` refuses writes into paths reserved by declared state:

<!-- shown: writing where a struct lives -->
```rust
let said = match kv.namespace("network").set("port", &"8080".to_string()) {
    Err(KvWrite::Declared {
        at,
        declared_at,
        by,
    }) => {
        format!("{at} lies in {declared_at}, which {by} declared")
    }
    other => {
        other?;
        "the write went through".to_string()
    }
};

kv.namespace("networkish")
    .set("port", &"8080".to_string())?;
```
<!-- /shown -->

The error formats as a single line:

<!-- printed: writing where a struct lives from book_kv -->
```
network.port is declared by Network
```
<!-- /printed -->

You can also match its fields: `at` is the attempted write path,
`declared_at` is the conflicting declared path, and `by` identifies the struct:

<!-- printed: what the caller can say about it from book_kv -->
```
network.port lies in network.port, which Network declared
```
<!-- /printed -->

A key named `networkish` does not conflict with `network`: ownership is based
on path segments, not string prefixes.

Opening a `cell` or `map` over a reserved location returns `OpenStruct::Taken`
rather than the raw-write error above.

These checks prevent raw access from replacing a declared field with an
incompatible value. Such a replacement would leave the live field displaying
its previous value and could prevent the struct from opening on the next run.

## One path, one type

<!-- shown: asking for one path as two types -->
```rust
let ui = kv.namespace("ui");

let _width = ui.cell("width", 800u32)?;
let refused = ui.cell("width", String::new());
```
<!-- /shown -->

`Kv` does not keep a type registry. Creating a cell reads the stored value;
a second cell requesting an incompatible type fails to decode it.

This check also works after a restart because the stored value remains.
If the path is empty, the first cell writes its default. Raw `set` is different:
it can replace a value with another type.

```rust
kv.set("thing", &1u32)?;
kv.set("thing", &"now a string".to_string())?;   // replaces it
kv.get::<u32>("thing")?;                          // Err, at the read
```

After that replacement, a `get` requesting the old type returns a decode error.

## Schema and migrations

Versioned schemas, generated migrations and drift checking belong to declared
state structs. Use a struct instead of raw keys when you need to evolve a known
data layout across releases.

## Where it lands in the file

redb and SQLite store a `Kv` path as a key, like any other path.

In JSON, TOML and RON documents, declared structs form nested trees. Undeclared
paths are stored as **complete keys** alongside those trees, rather than split
into document members:

<!-- shown: a declared struct and a Kv write, side by side in one file -->
```rust
store
    .kv()
    .namespace("plugins")
    .namespace("left")
    .set("width", &240u32)?;
```
<!-- /shown -->

<!-- printed: the file a Kv write leaves from book_kv -->
```
{
  "chrome": {
    "width": 800
  },
  "plugins.left.width": 240
}
```
<!-- /printed -->
