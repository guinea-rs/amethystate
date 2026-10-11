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

`amethystate` is a state manager for Rust applications: state is declared as an ordinary struct, its fields are
reactive, and they outlive the program.

### Features

- **Struct-defined state** — one attribute turns a struct's fields into persisted reactive ones, with defaults and subscriptions
- **Rules on a value** — one `fn` on a field puts right every value it takes, read or written, and the correction reaches the field and the file alike; an invariant between fields goes in the struct's own `Open`
- **Runtime-defined keys** — a map entry or a `Kv` path gets the same subscriptions and durability as a declared field
- **Read and write every frame** — writes are buffered and batched, reads answer from memory
- **Durable when it matters** — `durable()` on a field, a map or a `Kv` path returns only once the value is on disk, for the writes that must not sit in a buffer
- **Behaviour you choose** — which engine holds the state, when a write reaches the disk, what a field does with a value that will not read, what a new version does to an old file
- **Migrations** — explicit versions, run at startup; drift is logged
- **Engines** — `redb`, `sqlite`, and text as `json`/`toml`/`ron` on disk; text files reload on external edits
- **In a browser** — `localStorage`; a page hears the writes another tab makes
- **[guinea](https://guinea-rs.github.io/amethystate/integrations/guinea)** — a timer whose period follows a stored setting
- **Tracing** — structured events, each write tagged with its source struct

> [!WARNING]
> **About half of the engines are not used in any real application.**
> They run in CI and were smoke-tested, so expect rough edges there, and report what you hit.

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

The API is broad, and a guess from the signatures seldom lands on the answer it already has. Before writing code against amethystate, read **[AGENTS.md](AGENTS.md)** and the book it points to.

### Compatibility
The minimum supported Rust version (MSRV) for `amethystate` is **1.90**.

### License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
