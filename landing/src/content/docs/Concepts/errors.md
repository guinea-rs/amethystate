---
title: Handling errors
sidebar:
  label: Errors
  order: 20
---

Operations return error enums specific to that operation. For example,
`OpenStruct` describes failures when opening a state struct, while `KvWrite`
describes errors from raw path access. You can match the relevant cases without
handling variants that cannot occur at that call.

These types implement `std::error::Error`. Match individual variants when you
can recover, or propagate the error with `?`.

## Taking a refusal apart

<!-- shown: telling one refusal from another -->
```rust
let refused = match Panel::try_new_with(&store) {
    Ok(_) => return Ok(()),
    Err(why) => why,
};

let said = match refused {
    OpenStruct::WillNotRead { at, why } => format!("{at} holds something else: {why}"),
    OpenStruct::Taken(taken) => format!("{} already holds it", taken.held_by),
    OpenStruct::NotAPath(why) => format!("that is not a path: {why}"),
    OpenStruct::Store(disk) => format!("the store: {disk}"),
};
```
<!-- /shown -->

The match has no `_` arm because it handles every variant. These error enums
are exhaustive: if a future version adds a variant, the compiler will identify
matches that need updating.

Variants include context for diagnosing the failure. `OpenStruct::Taken`, for
example, identifies the requested path and struct as well as the conflicting
path and its owner. Match the fields you need and ignore the rest.

## Handing it on

<!-- shown: letting the caller's own error type take it -->
```rust
fn with_anyhow(store: &amethystate::Store) -> anyhow::Result<()> {
    store.set(["ui", "width"], &800u32)?;
    Ok(())
}

fn with_a_box(store: &amethystate::Store) -> Result<(), Box<dyn Error + Send + Sync>> {
    store.set(["ui", "height"], &600u32)?;
    Ok(())
}
```
<!-- /shown -->

Errors are `Send + Sync + 'static`, format as a single line through `Display`,
and expose their underlying cause through `source()`. They can be propagated
into `anyhow`, `eyre` or `Box<dyn Error>` with `?`.

## The sets

| set | raised by |
| --- | --- |
| `OpenStruct` | `try_new`, `try_new_with`, `try_load`, `try_load_with`, `Open::try_new_with`, `AmeStateSlice::try_load_slice`, `new_with_id`, `new_with_id_under`, `Schema::open`, `Kv::cell` |
| `OpenStore` | `StoreBuilder::build`, `migrate`, `located` |
| `LoadMap` | `Kv::map`, and a map field's own constructor |
| `ReadValue` | `Store::get` |
| `WriteValue` | `Store::set`, `Store::delete`, `Field::durable` writes, `ReactiveCell::set`/`update`/`modify`, `ReactiveMap::insert` |
| `KvWrite` | `Kv::get`, `Kv::set`, `Kv::remove` |
| `ScanKeys` | `Store::scan_keys`, `Store::scan_prefix`, `Kv::keys` |
| `Flush` | `save_now`, `close`, `flush_prefix` |
| `RunStep` | every `MigrationContext` method, and what a migration step hands back |

These types deliberately differ. `WriteValue` can report `Absent` and
`SourceGone`, while a raw `Kv` call can report `Declared`. Separate enums keep
unreachable cases out of a caller's match.

Plain constructors such as `new_with` panic on errors returned by their `try_`
variants. `Field::set`, `update` and `modify` also have no error result:
unencodable values panic, closed-store writes are ignored with a warning,
and background saving failures are reported by store observers.
Use `Field::durable` when a write needs a `Result`.

Maps have their own opening errors for invalid entry paths and unreadable key
names. Migration steps distinguish incompatible records from storage failures
so the step can handle them differently.

`Field::try_get` instead returns a `Disagreement`: it describes a known problem
with the cached field's view of storage, not a new storage operation.
See [Unreadable values](/amethystate/state/unreadable-values/).

## The report is still there

Storage-backed variants preserve an `error_stack::Report`, not just a message.
This includes `Store`, `WillNotRead`, `WillNotEncode` and `TooDeep` errors.

<!-- shown: the report under a variant that named the failure -->
```rust
let refused = store.get::<u16>(["port"]).unwrap_err();

let amethystate::store::ReadValue::WillNotRead { at, why } = refused else {
    panic!("the bytes are there and they are not a u16")
};
```
<!-- /shown -->

A report contains:

- **Contexts:** error categories added as the failure passes through layers.
- **Attachments:** typed facts such as paths, file names and limits.

Extract a report by matching the outer error:

<!-- shown: getting at the report a set carries -->
```rust
fn what_the_store_said(why: LoadMap) -> StorageResult<()> {
    match why {
        LoadMap::EntryWillNotRead { why, .. } => Err(why.into_report()),
        other => panic!("an entry was expected to be at fault: {other}"),
    }
}
```
<!-- /shown -->

### The top names the operation

<!-- shown: what a failure says it is -->
```rust
let refused = store.kv().map::<String, u64>("labels").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let context = report.current_context();
let sentence = report.to_string();
```
<!-- /shown -->

Here `context` is `StorageError::Codec`, and `sentence` is
*the value could not be encoded or decoded*.

`current_context()` returns the outermost error category. `StorageError`
identifies the operation, such as `Write`, `Scan` or `Codec`; deeper frames
preserve the engine-specific cause.

Unlike the operation error enums, `StorageError` is `#[non_exhaustive]`.
Include a catch-all when matching it.

### The particulars are attached as types

`amethystate::errors::facts` contains typed attachments for keys, scan prefixes,
entries, store files and value sizes. Each wraps its data and formats it with
a descriptive label.

`facts::all::<T, _>` retrieves all attachments of a type, innermost first:

<!-- shown: reaching the entry that failed -->
```rust
let refused = store.kv().map::<String, u64>("ports").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let entries: Vec<&Entry> = facts::all::<Entry, _>(&report).collect();
let prefixes: Vec<&Prefix> = facts::all::<Prefix, _>(&report).collect();
```
<!-- /shown -->

`entries` contains `Entry("http")`, and `prefixes` contains `Prefix("ports")`.
These identify the map entry that failed; a one-line error message does not
necessarily include every attachment.

A lookup for an attachment not present in this report returns nothing:

<!-- shown: asking for a fact the report does not carry -->
```rust
let refused = store.kv().map::<String, u64>("ports").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let key = facts::all::<Key, _>(&report).next();
```
<!-- /shown -->

`key` is `None` because this failure concerns an entry under a prefix,
not a separately attached key.

Attachments depend on what each error layer knew. They are diagnostic evidence,
not a fixed record guaranteed to contain every field. They are constructed only
on the error path.

Use reports for logging and diagnostics; prefer the outer error variants when
choosing a recovery action.

## Printing one

`Display` (`{}`) formats the operation error as one line. `Debug` (`{:?}`) on
its underlying report includes the context chain and typed facts:

<!-- shown: an entry that will not decode -->
```rust
store.set(["ports", "http"], &"text".to_string())?;

let undecodable = store.kv().map::<String, u64>("ports").unwrap_err();
```
<!-- /shown -->

<!-- printed: an entry that will not decode from book_errors -->
```
the entry at ports.http will not read back

the value could not be encoded or decoded
├╴as: u64
├╴value bytes: 5
├╴prefix: ports
├╴entry: http
│
├─▶ Erased codec error: wrong msgpack marker FixStr(4)
│
╰─▶ wrong msgpack marker FixStr(4)
```
<!-- /printed -->

<!-- shown: an entry whose name is not the map's key type -->
```rust
let wrong_key = store.kv().map::<Id<u16>, String>("ports").unwrap_err();
```
<!-- /shown -->

<!-- printed: an entry whose name is not the map's key type from book_errors -->
```
`http` under ports will not read as a Id<u16>
```
<!-- /printed -->

This variant needs no underlying report: it already contains the map,
entry and requested key type.

<!-- shown: a path that names nothing -->
```rust
let names_nothing = store.set(Vec::<String>::new(), &1u32).unwrap_err();
```
<!-- /shown -->

<!-- printed: a path that names nothing from book_errors -->
```
the write was given no path to land at: a path of no levels names the whole store - say `StorePath::root()` to mean it
```
<!-- /printed -->

<!-- shown: a path past the cap it was given -->
```rust
let shallow = StoreBuilder::new(settings)
    .limits(|l| l.key_depth(4))
    .build()?;

let too_deep = shallow.set(["a", "b", "c", "d", "e"], &1u32).unwrap_err();
```
<!-- /shown -->

<!-- printed: a path past the cap it was given from book_errors -->
```
a.b.c.d.e is deeper than this store reads back

deeper than this store reads back
├╴key: a.b.c.d.e
├╴levels: 5, and the limit is 4
├╴set by: limits(|l| l.key_depth(..))
╰╴what is stored here spends the same budget - this store reads 512 levels in all
```
<!-- /printed -->

The next report separates a short description from details such as the limit
and the amount used. Use that structure in diagnostics: a readable summary
with enough attached data to investigate.

## When a bound asks for `std::error::Error`

`Report` does not itself implement `std::error::Error`. The operation error
enums do, so ordinary `?` propagation works. If you extract a report and need
to pass it through an `E: Error` boundary, convert it with `into_error`:

<!-- shown: turning a report into a std error -->
```rust
let std_error = report.into_error();

let sentence = std_error.to_string();
let whole = format!("{std_error:?}");
```
<!-- /shown -->

The wrapper retains the report: `sentence` stays the same and `whole` still
contains `entry: http`. Use `as_error` to borrow the corresponding error view
without consuming the report.

`error_stack` is re-exported as `amethystate::error_stack`, and `Report` is also
available through `amethystate::errors`. These re-exports let your code use
the same version as the library.
