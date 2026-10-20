---
title: "ReactiveCell<T>"
sidebar:
  label: ReactiveCell
  order: 10
---

`ReactiveCell<T>` provides the same read, write and subscription API for a field,
a map entry, a dynamic path or an in-memory value. A component can accept a cell
without knowing which struct, map or store the value comes from.

<!-- shown: four ways to reach a cell -->
```rust
let width = state.sidebar_width().cell();
let cpu_column = state.widths().entry_cell("cpu".to_string());
let by_path = store.kv().cell("dragging", 0u64)?;
let loose = ReactiveCell::new_volatile(0u64);

let mut columns: HashMap<String, ReactiveCell<u64>> = HashMap::new();
columns.insert("sidebar".to_string(), width);
columns.insert("cpu".to_string(), cpu_column);
columns.insert("dragging".to_string(), by_path);
columns.insert("loose".to_string(), loose);
```
<!-- /shown -->

The first three examples write through to the store. `ReactiveCell::new_volatile`
keeps its value only in memory. A cell created from an `#[amestate(volatile)]`
field is also in-memory only.

`kv.cell` creates a cell from a path and default without a state declaration.
It reads existing data rather than registering a type. Requesting an
incompatible type at the same path produces a decode error; compatible types
are not distinguished by a separate type registry.
[Kv](/amethystate/primitives/kv/#one-path-one-type).

## Reading

```rust
fn get(&self) -> Option<T>
```

`get` returns `None` if a map entry is absent or the cell's source has been
dropped. Unlike [`Field`](/amethystate/primitives/field/), a cell does not
always have a value to return.

Reads use the cell's cache rather than reading storage, so you can call `get`
from a render loop.

<!-- shown: reading, writing and watching a cell -->
```rust
let current = cell.get();

let _sub = cell.subscribe(|width| println!("width -> {width:?}"));

cell.set(200)?;
cell.update(|width| width + 10)?;
cell.modify(|width| *width += 10)?;
```
<!-- /shown -->

## Writes land where the value lives

A cell forwards writes to its source. A stored field is updated in the store;
a volatile source is updated in memory.

The cell updates its cache after the source accepts the change. If a write is
rejected, `get` continues to return the previous value. Acceptance here does
not mean the value has already reached disk.

## A cell onto a map entry

<!-- shown: a cell onto a map entry -->
```rust
state.widths().insert("cpu".to_string(), &120)?;

let cpu = state.widths().entry_cell("cpu".to_string());
let absent = state.widths().entry_cell("gpu".to_string());

assert_eq!(cpu.get(), Some(120));
assert_eq!(absent.get(), None);

state.widths().remove("cpu")?;

assert_eq!(cpu.get(), None);
assert!(cpu.set(80).is_err());
```
<!-- /shown -->

An entry cell is empty while the key is absent, including after removal.
`set`, `update` and `modify` then return `WriteValue::Absent` with the entry's
path. Reinsert the key through the map before writing through its cell.

## What a cell keeps alive

There are two lifetime choices:

- `cell()` and `entry_cell()` hold a **weak reference** to the source. They do
  not keep it alive. Once its last owning handle is dropped, reads return `None`
  and writes return `WriteValue::SourceGone`.
- `into_cell()` and `into_entry_cell()` **keep the source alive** by retaining
  an owning handle. Use them when the cell is stored in a component or passed
  somewhere that may outlive the original state struct.

Getters such as `state.sidebar_width()` clone a handle; they do not remove the
field from the struct. Converting that handle with `into_cell()` gives the cell
shared ownership of the same source.

<!-- shown: a view, and a cell that owns what feeds it -->
```rust
let view = state.sidebar_width().cell();
let owned = state.sidebar_width().into_cell();

drop(state);
```
<!-- /shown -->

Both cells refer to the same field. The owning cell keeps it alive after the
struct is dropped, so the weak cell still reads a value. Once the owning cell
is dropped too, the weak cell returns `None`.
