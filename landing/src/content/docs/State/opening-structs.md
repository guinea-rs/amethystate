---
title: Opening a struct your own way
sidebar:
  label: Opening a struct
  order: 5
---

Every struct with a place of its own implements two traits. `Schema` opens it
exactly as declared: the fields read, their rules run, the place claimed. `Open`
is how the rest of the program opens it, and unless the struct says otherwise
it is `Schema` and nothing more. Code that opens structs it does not name asks
for `Open`:

<!-- shown: opening any struct that has a place of its own -->
```rust
fn opened<S: Open>(store: &Store) -> Result<S, OpenStruct> {
    S::new_with(store)
}
```
<!-- /shown -->

## `open = manual`

`open = manual` is for a struct whose opening is more than its declaration - a
value worked out from the machine it runs on, a check against something the
application knows. The macro then writes no `Open`, and the struct's author
does, starting from `Schema`:

<!-- shown: a struct that opens its own way -->
```rust
#[amethystate(prefix = "agent", open = manual)]
pub struct AgentSettings {
    #[amestate(default = 0u32)]
    pub workers: u32,
}

impl Open for AgentSettings {
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>()?;
        let settings = <Self as Schema>::open(store)?;

        if settings.workers().get() == 0 {
            settings.workers().set(machine.cores)?;
        }

        Ok(settings)
    }
}
```
<!-- /shown -->

What it needs comes from `store.context()`, the same values a declared
[rule](/amethystate/state/rules/) is handed, and `?` turns a missing one - or
any `Invalid` of its own - into `OpenStruct::Declined`, which carries the
reason as it was written.

## An invariant between fields

A field's rule sees one value and none of its siblings, and the struct is the
place its fields are kept rather than a value with a rule of its own. So an
invariant between fields goes here. The hand-written `Open` sees every field at
once, after each has passed its own rule:

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
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        let window = <Self as Schema>::open(store)?;

        if window.min().get() > window.max().get() {
            return Err(Invalid::new("the smallest window is wider than the largest").into());
        }

        Ok(window)
    }
}
```
<!-- /shown -->

It runs once, where the struct is opened. A write to `min` afterwards is judged
by `min`'s own rule and by nothing that knows about `max`; two fields written
from two threads are a limit the
[Durability](/amethystate/concepts/durability/#concurrent-access) page sets out.

## The only way in

The hand-written `Open` is the only way in. `new_with` on the struct, `load_with`
on a persistent one and `AmeStateSlice::load_slice` all go through it, so no
caller opens the struct past it by accident. That is also why the impl
itself starts from `<Self as Schema>::open` and never from `Self::new_with`:
the second is this very `Open`, and would call itself. And a struct that says
`open = manual` has no `new()` or `load()` over the global store: reaching for
the global store is a compile error rather than something a review has to
catch.

`open` belongs to a struct with a place of its own. A nested struct is opened
by the struct holding it, so the macro refuses `open` on one.
