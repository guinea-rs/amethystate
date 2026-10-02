---
title: When a value will not read
sidebar:
  order: 3
---

Three moments, each with its own answer: a struct opening over bytes that will
not decode, a key deleted under a live field, and a change arriving that will
not decode. A value that decodes perfectly and is still nonsense is a different
matter, and [Rules](/amethystate/state/rules/) is where it is answered.

## Opening

A declared path holding something that will not decode into the field's type
refuses construction and names the path. That is `Refuse`, the default.
`UseDefault` is for the application that has to start anyway: the field takes
its declared default, the stored value stays on disk for somebody to fix, and
[`try_get`](/amethystate/primitives/field/) answers `Err` from construction
until a change decodes.

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

**A field may tighten what its struct wrote.** Above, the settings open with a
broken `port`, and a `licence` that will not read stops the whole thing.
`Refuse` on the struct with `UseDefault` on a field is a compile error naming
the field. A `nested` struct inherits its holder's answer, tightens it the same
way, and is checked against the holder while it compiles.

**Where nothing said, the store does.** What a field says wins over what its
struct says, and the struct over what the store was opened with — so an
application says once what it wants of everything that had no opinion, and
every declaration above stands untouched:
[Opening a store](/amethystate/store/opening/).

## A key deleted under a live field

The field goes on reporting what it last held: that is what was on screen a
moment ago, and the declared default is a compile-time guess. `UseDefault` asks
for the guess:

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

The field keeps the last value the store agreed with and no subscriber is
called. `try_get` reports it, and clears itself as soon as a change decodes.
There is nothing to declare here.
