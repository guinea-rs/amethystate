---
title: Unreadable values
sidebar:
  order: 3
---

Unreadable data is handled differently during construction and after a field
is already open. Deleting a key is a third, separate case.
If a value decodes but needs correction, use a [rule](/amethystate/state/rules/).

## Opening

By default, an unreadable stored value prevents the struct from opening.
`new_with` panics with the path and codec error; `try_new_with` returns the
error. This policy is called `Refuse`.

With `UseDefault`, construction succeeds and the field displays its declared
default. The original data is preserved for recovery, and
[`try_get`](/amethystate/primitives/field/) reports the unreadable value until
a readable update arrives:

<!-- shown: a struct that opens over a value it cannot read -->
```rust
#[amethystate(prefix = "mixed", on_unreadable = UseDefault)]
pub struct Mixed {
    #[amestate(default = 8080u16)]
    pub port: u16,

    #[amestate(default = "".to_string(), on_unreadable = Refuse)]
    pub licence: String,
}
```
<!-- /shown -->

**A field can make the policy stricter.** In this example, an unreadable `port`
uses a default, but an unreadable `licence` prevents construction. A `Refuse`
struct cannot contain a field explicitly opting into `UseDefault`; the macro
rejects that combination. Nested structs inherit and can tighten the parent's
policy in the same way.

**Unspecified policies come from the store.** Field declarations override
struct declarations, which override store settings. This lets an application
set a default policy without replacing explicit choices in its state types.
See [Opening a store](/amethystate/store/opening/).

**Match errors by variant, not message.** `try_new_with` returns `OpenStruct`:

<!-- shown: telling one failed open from another -->
```rust
match Strict::try_new_with(&store) {
    Ok(_) => {}
    Err(OpenStruct::WillNotRead { at, why }) => eprintln!("{at} is unreadable: {why}"),
    Err(OpenStruct::Taken(taken)) => {
        eprintln!("{} already holds {}", taken.held_by, taken.at)
    }
    Err(other) => return Err(other.into()),
}
```
<!-- /shown -->

With `UseDefault`, construction succeeds, but the field exposes the path and
reason through a `Disagreement`. It describes the difference between the
cached value and storage, not a new read operation:

<!-- shown: asking a field what the store disagrees with -->
```rust
let held = match state.port().try_get() {
    Ok(port) => port,
    Err(no) => {
        match no.reason {
            Reason::WillNotRead(said) => eprintln!("{} will not decode: {said}", no.at),
            Reason::Occupied(said) => eprintln!("{} was already taken: {said}", no.at),
            _ => eprintln!("{} is not what the store has", no.at),
        }

        state.port().get()
    }
};
```
<!-- /shown -->

The reason identifies the problem:

- `WillNotRead`: the codec could not decode the stored value.
- `Occupied`: the field could not write its default because the path already
  contained data.
- `Reason::Closed`: the store is closed; the field retains its cached value.
- `Reason::NotWrittenBack`: a [rule](/amethystate/state/rules/) corrected a stored
  value, but the correction could not be written back.

`Reason` is `#[non_exhaustive]`, so include a `_` arm when matching it.
The operation error enums use a different policy; see
[Errors](/amethystate/concepts/errors/).

## A key deleted under a live field

By default, `on_delete = Keep` retains the field's current value when its key
is deleted. Select `UseDefault` to reset the field instead:

<!-- shown: a field that wants the default back when its key goes -->
```rust
#[amethystate(prefix = "mixed_delete")]
pub struct MixedDelete {
    #[amestate(default = 800u32)]
    pub width: u32,

    #[amestate(default = 600u32, on_delete = UseDefault)]
    pub height: u32,
}
```
<!-- /shown -->

## A live change that will not decode

If an update cannot be decoded, the field retains its last readable value and
does not notify subscribers of the invalid update. `try_get` reports the
problem until a readable update arrives. No additional policy is required.
