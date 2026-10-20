---
title: Opening a struct your own way
sidebar:
  label: Opening a struct
  order: 5
---

A state struct with its own storage location implements two traits:

- `Schema` loads the declared fields, runs their rules and claims their paths.
- `Open` is the constructor interface used by application code. Its generated
  implementation delegates to `Schema`, but you can replace it with your own.

Use an `Open` bound in generic code that constructs state structs:

<!-- shown: opening any struct that has a place of its own -->
```rust
fn opened<S: Open>(store: &Store) -> S {
    S::new_with(store)
}
```
<!-- /shown -->

## `open = manual`

Use `open = manual` when construction needs additional work, such as adjusting
settings to the current machine or checking them against application data.
The macro stops generating `Open`; implement it yourself and call `Schema`
to perform the normal loading first:

<!-- shown: a struct that opens its own way -->
```rust
#[amethystate(prefix = "agent", open = manual)]
pub struct AgentSettings {
    #[amestate(default = 0u32)]
    pub workers: u32,
}

impl Open for AgentSettings {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>();
        let settings = <Self as Schema>::open(store)?;

        if settings.workers().get() == 0 {
            settings.workers().set(machine.cores);
        }

        Ok(settings)
    }
}
```
<!-- /shown -->

Implement `try_new_with`, which returns a `Result`. The trait provides
`new_with`, which panics if that result is an error. Read dependencies from
`store.context()`, as you would in a [rule](/amethystate/state/rules/).
`require` panics if the required type was not supplied when building the store.

## An invariant between fields

A field rule only sees that field's value. Put checks involving several fields
in `Open`, where all fields are available and their individual rules have
already run:

<!-- shown: an invariant between fields, checked where the struct is opened -->
```rust
#[amethystate(prefix = "window", open = manual)]
pub struct Window {
    #[amestate(default = 400u32)]
    pub min: u32,

    #[amestate(default = 1600u32)]
    pub max: u32,
}

impl Open for Window {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let window = <Self as Schema>::open(store)?;

        if window.min().get() > window.max().get() {
            window.max().set(window.min().get());
        }

        Ok(window)
    }
}
```
<!-- /shown -->

This check runs only when the struct opens. Later writes to `min` run its field
rule, not this cross-field check. Your application must preserve the invariant
during subsequent updates. See also
[Concurrent access](/amethystate/concepts/durability/#concurrent-access).

## The only way in

The generated constructors use your `Open`: `new_with` and `try_new_with`,
persistent-mode `load_with` and `try_load_with`, and loading through
`AmeStateSlice`.

Inside the implementation, call `<Self as Schema>::open`, not
`Self::try_new_with`: the latter would recursively call the same `Open`.
With `open = manual`, the macro also omits global-store `new()` and `load()`
constructors, so callers must provide a store explicitly.

`open` belongs to a struct with a place of its own. A nested struct is opened
by the struct holding it, so the macro refuses `open` on one.
