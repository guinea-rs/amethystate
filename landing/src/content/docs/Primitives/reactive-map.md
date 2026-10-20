---
title: "ReactiveMap<K, V>"
sidebar:
  label: ReactiveMap
  order: 9
---

`ReactiveMap<K, V>` stores entries under separate paths. Keys can be added at
runtime, and subscriptions can watch the whole map or one entry. Unlike a
`HashMap` stored in a single field, each entry can be updated independently.

## Reading

<!-- shown: reading a map -->
```rust
let one: Option<u64> = widths.get("cpu");

if let Some(width) = widths.get("cpu") {
    println!("cpu is {width}");
}

let there = widths.contains_key("mem");
let how_many = widths.len();

for key in widths.keys() {
    println!("{key}");
}

for (key, value) in widths.entries() {
    println!("{key}: {value}");
}
```
<!-- /shown -->

`len` reads an in-memory counter, including changes that have not reached disk.
`keys` and `entries` iterate over the map's in-memory state; `keys` does not
deserialize values.

### Looking without taking

`entries` clones the values it returns. To borrow them instead, create a `view`
and call its `iter()` method:

<!-- shown: looking at a map without cloning it -->
```rust
let held = widths.view();
let total: u64 = held.iter().map(|(_, width)| width).sum();

let mut widest = 0;
for (_, width) in &widths.view() {
    widest = widest.max(*width);
}
```
<!-- /shown -->

A `for` loop keeps the temporary view alive for the loop. Bind the view to a
variable if you need to use it outside that expression.

### A walk owns a version

`view`, `entries` and `keys` capture the map's current version. Iteration does
not hold a lock against writers, so you can update the map while reading it,
including from the same thread:

<!-- shown: walking a map and writing to it -->
```rust
let mut walked = Vec::new();

for (key, width) in widths.entries() {
    widths.remove(&key)?;
    walked.push((key, width));
}

assert_eq!(walked, [("cpu".to_string(), 120), ("mem".to_string(), 80)]);
assert!(widths.is_empty());
```
<!-- /shown -->

The iterator still visits every original key: removals affect the current map,
not the version being iterated.

A view retained for a frame gives consistent reads even if the map changes
between them. It shares unchanged data with the map rather than copying the
whole collection, but keeps old data alive until the view is dropped.

## Writing

<!-- shown: adding, changing and removing an entry -->
```rust
widths.insert("cpu".to_string(), &120)?;

widths.update("cpu", &200)?;
widths.modify("cpu", |width| *width += 10)?;

let absent = widths.update("gpu", &90);

widths.remove("cpu")?;
widths.clear()?;
```
<!-- /shown -->

`insert` adds or replaces an entry. `update` requires an existing key and
returns `Absent` otherwise; its `MapChange::Update` event needs a previous value.
`modify` accepts a mutable reference so you can change a value without
constructing its replacement yourself.

`remove` emits one `MapChange::Remove` with the writing handle's ID.
`clear` removes the map's contents with one store deletion and one map event,
rather than an event per entry.

Use `upsert` when the new value depends on whether the key exists:

<!-- shown: writing an entry whether or not it is there -->
```rust
for page in ["home", "about", "home"] {
    hits.upsert(page, |seen| seen.map_or(1, |count| count + 1))?;
}

let now = hits
    .durable()
    .upsert("home", |seen| seen.map_or(1, |count| count + 1))?;
```
<!-- /shown -->

The closure receives `Some(previous)` or `None`. Its result is stored and
returned to the caller. The key is borrowed, so updating an existing key does
not require constructing a new `String`. Subscribers receive `Insert` for a
new key and `Update` for an existing one.

Like `modify`, this is a non-atomic read-modify-write operation: a concurrent
write between the read and write can be overwritten.

To remove selected entries, iterate over a captured version and remove them:

```rust
for (key, width) in widths.entries() {
    if width < 80 {
        widths.remove(&key)?;
    }
}
```

## What a key may be

`ReactiveMapKey` requires `AsRef<str>`. String keys can therefore be looked up
without formatting or parsing them.

Wrap a non-string key in `Id`, which formats the value once and retains that
string:

```rust
let ports = store.kv().map::<Id<u16>, bool>("ports")?;

ports.insert(Id::new(8080), &true)?;
assert_eq!(ports.get("8080"), Some(true));
```

`Id::new` computes the string representation when the key is constructed.
Later lookups reuse it, avoiding repeated formatting and allocation for values
such as UUIDs. `get` returns that representation.

An empty string is also a valid key and supports the usual read, write and
listing operations.

## What order entries come back in

Entries are sorted by their string keys, in the same order as a store listing:

<!-- shown: the order entries come back in -->
```rust
counts.insert("9".to_string(), &1)?;
counts.insert("10".to_string(), &1)?;
counts.insert("a.b".to_string(), &1)?;
counts.insert("a1b".to_string(), &1)?;

let order: Vec<String> = counts.keys().collect();
assert_eq!(order, ["10", "9", "a.b", "a1b"]);
```
<!-- /shown -->

String order is not numeric order: `"10"` sorts before `"9"`, including for
`Id<u16>`. Literal separators are compared as part of the key: `a.b` sorts
before `a1b`. Internal path escaping does not change that order.

The map does not retain insertion order. For ordered columns or list items,
keep a separate order and use the map for lookup.

## Hearing about a change

<!-- shown: hearing about a change -->
```rust
let _all = widths.subscribe_any(|change| {
    println!("{change:?}");
});

let _cpu = widths.subscribe_key("cpu".to_string(), |change| {
    println!("cpu: {change:?}");
});

widths.insert("cpu".to_string(), &120)?;
widths.insert("mem".to_string(), &80)?;
```
<!-- /shown -->

`subscribe_any` receives changes to any entry; `subscribe_key` watches one key.
Both return a guard whose lifetime controls the subscription.

Use `subscription_with()` to combine key filtering, source filtering or a
stream. See [Subscriptions](/amethystate/concepts/subscriptions/).

### What `external` filters

On a map, `.external()` filters this handle's own **Update** events only.
`Insert`, `Remove` and `Clear` still reach its subscribers: adding or removing
a key changes the collection a UI needs to display, regardless of the writer.

Consequently, `insert` on a new key is delivered to an external-only
subscription, but `insert` on an existing key is filtered as an update.
[Subscriptions](/amethystate/concepts/subscriptions/#whose-write-was-it)
explains how handle IDs identify writers.

## One entry as a cell

Use `entry_cell(key)` to pass a single entry as a
[`ReactiveCell`](/amethystate/primitives/reactive-cell/) to code that does not
need access to the map.
