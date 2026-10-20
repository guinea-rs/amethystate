<div align="center">

<img src="https://raw.githubusercontent.com/guinea-rs/amethystate/master/logo.svg" alt="amethystate" width="384" />

# amethystate

[![Crates.io](https://img.shields.io/crates/v/amethystate.svg)](https://crates.io/crates/amethystate)
[![Docs.rs](https://docs.rs/amethystate/badge.svg)](https://docs.rs/amethystate)
[![CI](https://github.com/guinea-rs/amethystate/actions/workflows/ci.yml/badge.svg)](https://github.com/guinea-rs/amethystate/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-yellow.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-orange.svg)](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/)

*A state manager for Rust applications.*

</div>

`amethystate` manages persistent, reactive state for Rust applications.
Declare your settings as a struct, read and update them through field handles,
and subscribe to changes. The library saves the values between runs.

### Features

- **State declared as structs** — fields have defaults, persistent values and subscriptions.
- **Rules for values** — normalize a value whenever it is read or written, such as clamping a window size. Use a custom `Open` implementation for checks involving several fields.
- **Dynamic keys** — use a reactive map or `Kv` when the keys are not known at compile time.
- **Buffered writes** — read from memory and update state without waiting for each change to reach disk.
- **Explicit durability** — use `durable()` on a field, map or `Kv` path when a write must reach disk before returning.
- **Configurable storage** — choose the engine, flush timing and policies for unreadable values.
- **Migrations** — version your structs and transform old data when opening the store. Unversioned schema changes are reported as drift.
- **File engines** — redb, SQLite, JSON, TOML and RON. Text engines also reload changes made outside the application.
- **Browser storage** — persist to `localStorage` and receive changes from other tabs.
- **[guinea integration](https://guinea-rs.github.io/amethystate/integrations/guinea)** — drive a timer's period from a stored setting.
- **Tracing** — structured events identify the struct responsible for a write.

> [!WARNING]
> **About half of the engines are not used in any real application.**
> They are covered by CI and smoke tests, but that is not a substitute for
> use in an application. Expect rough edges and report problems you find.

<!-- shown: the readme's first example -->
```rust
use amethystate::{StoreBuilder, amethystate};

#[amethystate(prefix = "network")]
pub struct NetworkState {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = StoreBuilder::new("./app").build()?;
    let state = NetworkState::new_with(&store);

    let _sub = state.port().subscribe(|port| println!("port → {port}"));

    state.port().set(9090);

    Ok(())
}
```
<!-- /shown -->

---

See the **[book](https://guinea-rs.github.io/amethystate/introduction)** for full documentation — concepts, the store, and migrations.

### For agents

Before writing code against amethystate, read **[AGENTS.md](AGENTS.md)** and
the relevant book sections. The book explains when to use each API and the
trade-offs that are not visible in its signature.

### Compatibility
The minimum supported Rust version (MSRV) for `amethystate` is **1.90**.

### License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
