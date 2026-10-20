---
title: Quick Start
sidebar:
  order: 3
---

This example declares settings, opens a store and subscribes to a field.
Links throughout the page explain the options in more detail.

The prelude includes the types and traits used here:

```rust
use amethystate::prelude::*;
```

This includes `StoreExt` and `StoreBackend`: import these traits to call methods
such as `get` and `save_now`. The examples below use explicit imports to show
which item comes from which module.

## Declare the state

`#[amethystate]` generates persistent, reactive fields from a struct.
`prefix` sets their location in the store.

<!-- shown: declaring a state struct -->
```rust
use amethystate::amethystate;

#[amethystate(prefix = "network")]
pub struct NetworkState {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```
<!-- /shown -->

Defaults, nested structs, volatile fields, read policies, rules on a value and
serde interaction: the [State](/amethystate/state/defining-structs/) section.

## Open the store

<!-- shown: opening a store you hold yourself -->
```rust
let store = StoreBuilder::new(settings)
    .disk(|d| d.debounce(Duration::from_millis(500)))
    .build()?;

let state = NetworkState::new_with(&store);
```
<!-- /shown -->

Pass the store to `new_with` to create the state. You can also use a global
store or let the platform choose the data directory; see
[Opening a store](/amethystate/store/opening/) for these options and shutdown.

Timing, retries and what the store refuses to hold:
[Configuring a store](/amethystate/store/configuration/). Which engine holds
the file: [Installation](/amethystate/getting-started/installation/).

## Read, write, subscribe

<!-- shown: reading, writing and subscribing -->
```rust
println!("{}", state.host().get());

let _sub = state.port().subscribe(|port| {
    println!("port changed to {port}");
});

state.port().set(9090);
```
<!-- /shown -->

A write updates memory immediately; the store flushes it to disk later.
[Subscriptions](/amethystate/concepts/subscriptions/) explains callback threads,
filtering your own writes and subscription overhead.

To wait for the disk instead of the debounce:
[Durability](/amethystate/concepts/durability/).

## Keys you do not know at compile time

A map stores each entry at its own path, so entries can be added and observed
one at a time.

<!-- shown: a map whose keys are not known up front -->
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertThresholds {
    pub warning: u64,
    pub critical: u64,
}

#[amethystate(prefix = "sys")]
pub struct SystemSettings {
    #[amestate(default = {
        "cpu": AlertThresholds { warning: 70, critical: 90 },
        "mem": AlertThresholds { warning: 80, critical: 95 }
    })]
    pub limits: ReactiveMap<String, AlertThresholds>,
}
```
<!-- /shown -->

<!-- shown: working with a map -->
```rust
state.limits().insert(
    "gpu".to_string(),
    &AlertThresholds {
        warning: 60,
        critical: 85,
    },
)?;

let cpu = state.limits().get("cpu");

for (key, value) in state.limits().entries() {
    println!("{key}: {value:?}");
}

let _sub = state.limits().subscribe_any(|change| {
    println!("{change:?}");
});
```
<!-- /shown -->

`entries()` returns entries sorted by their string keys. Wrap non-string keys
in `Id`; these also sort by their string representation, so `Id<u16>` keys
can appear in the order `10, 100, 9`, not numeric order.

Use [Kv](/amethystate/primitives/kv/) for paths chosen at runtime without a
struct declaration. A [ReactiveCell](/amethystate/primitives/reactive-cell/)
provides a handle for reading, writing and subscribing to a single value.

## When the struct changes

Bumping a struct's `version` and declaring the steps between versions is how
data written by an older build is brought forward. A field renamed or retyped
*without* a bump is reported as drift and startup continues.

Open the store with `migrate` whenever `#[migrate]` is in the binary: `build`
runs no step at all.

[Migrations](/amethystate/migrations/overview/).

## Persistent-only mode

If you only need loading and saving, use `mode = "persistent"`. Fields remain
plain Rust values, and you save the struct explicitly. It has no subscriptions
and does not receive changes made elsewhere. This can suit an application that
already manages updates in its own event loop.

<!-- shown: a struct in persistent mode -->
```rust
#[amethystate(prefix = "kept", mode = "persistent")]
pub struct KeptSettings {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```
<!-- /shown -->

<!-- shown: writing a persistent struct -->
```rust
let mut state = KeptSettings::load_with(&store);

state.port = 9090;
state.save()?;

state.mutate(|d| {
    d.host = "0.0.0.0".to_string();
    d.port = 443;
})?;
```
<!-- /shown -->

`save_lazy` and `mutate_lazy` are the same two writes with the flush left to
the debouncer.

## What else there is

- **Rules** - normalize values read from storage or written through a field:
  [Rules](/amethystate/state/rules/).
- **Tracing** - structured events, each write tagged with the struct that made
  it: [Observability](/amethystate/concepts/observability/).
- **guinea** - a timer whose period follows a stored setting:
  [guinea](/amethystate/integrations/guinea/).
