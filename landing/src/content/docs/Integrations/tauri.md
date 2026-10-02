---
title: Tauri
---

`tauri-plugin-amethystate` is a Tauri v2 plugin that bridges your state structs to the Tauri frontend over IPC. It exposes commands for reading, writing, and subscribing to state, and ships a code generator that produces typed bindings for both TypeScript and Rust WASM frontends.

## Mental model

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (TypeScript or Rust/WASM)                         │
│  in-memory snapshot populated by load() and kept in         │
│  Field objects / structs                                    │
└────────────────────────┬────────────────────────────────────┘
                         │ IPC (Tauri commands)
┌────────────────────────▼────────────────────────────────────┐
│  amethystate (Rust)                                         │
│  in-memory write buffer, reactive subscriptions             │
└────────────────────────┬────────────────────────────────────┘
                         │ debounced flush / explicit save
┌────────────────────────▼────────────────────────────────────┐
│  Disk                                                       │
└─────────────────────────────────────────────────────────────┘
```

## Optimistic updates

The frontend API is synchronous by design — reads and writes return immediately without waiting for IPC confirmation. This means the frontend applies updates optimistically: the local value is updated first, and the IPC call to the backend follows asynchronously.

If the backend returns an error, the frontend value is reset to the last confirmed state. This tradeoff keeps the UI responsive but means a failed write will visibly revert.

## Installation

Add the plugin to your Tauri app's Rust crate:

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-amethystate = { version = "0.23", features = ["redb"] }
```

`amethystate` is re-exported as `tauri_plugin_amethystate::amethystate`, so no separate dependency is needed. The plugin's `redb`, `sqlite`, `json`, `toml` and `ron` features turn on the engine of the same name, and the store below opens only with one of them on.

Hand your store to the plugin in `main.rs`:

```rust
use tauri_plugin_amethystate::amethystate::StoreBuilder;

fn main() {
    let store = StoreBuilder::new("./app").build().unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_amethystate::init(store))
        .run(tauri::generate_context!())
        .unwrap();
}
```

## What the frontend reaches

The plugin answers for the places your structs declare, and for nothing else. When it starts, it takes every `#[amethystate]` struct with a prefix in the binary, the newest version of each line, and collects its fields, its maps with their entries, and the fields of the structs it holds. Every command refuses a key no struct declares. A `volatile` field is never among them. A read of a whole prefix, the root included, returns only the declared places under it.

A write is checked against the field's type before it reaches the store. A value that would not read back as that type is refused, so a frontend cannot leave behind something that keeps the application from opening next time. The key of a map entry is checked the same way, against the map's key type.

Where the host has the struct open, a field's declared [rule](/amethystate/state/rules/) judges a frontend write as it arrives, as it does any write by path. A value it corrects is written back, and the frontend hears the corrected one. A value it refuses stays in the store while the field keeps its last good value, and the frontend's write answers an error.

A field whose key goes, after a reset on the Rust side or a `Kv::reset_to_defaults`, tells its frontend so. Where the field takes its default again (`on_delete = UseDefault` on the field, on a struct around it, or in the store's `rules`), the frontend takes that default too. Where the field keeps its last value, so does the frontend.

## Permissions

Add the default permission set to `src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    "amethystate:default"
  ]
}
```

`amethystate:default` is the two sets below together:

| Set | Commands |
|-----|----------|
| `amethystate:read` | `get`, `get_prefix`, `scan_keys`, `subscribe`, `unsubscribe` |
| `amethystate:write` | `set`, `delete`, `delete_prefix`, `flush` |

A window that only shows state needs `amethystate:read`. `delete_prefix` is in `write` because a map's `clear()` is made of it; it reaches a field or a map, never a level above them.

Each command also has a permission of its own, `amethystate:allow-amethystate-<command>`, and a `deny-*` beside it that wins over any `allow-*`.

## Codegen

`amethystate-codegen` generates typed frontend bindings from your `#[amethystate]` structs. The binary must live in the same crate where your types are declared.

**1. Add the binary target and dependency:**

```toml
[[bin]]
name = "codegen"
path = "src/bin/codegen.rs"

[dependencies]
amethystate-codegen = { version = "0.23" }
```

For Rust WASM frontends, add the appropriate feature flag:

| Feature | Framework |
|---------|-----------|
| `leptos` | Leptos |
| `dioxus` | Dioxus |
| `yew` | Yew |
| *(none)* | vanilla WASM |

A TypeScript frontend needs no feature, but its value types come from `ts-rs`; the [TypeScript](/amethystate/integrations/typescript/) page walks through that binary.

**2. Create `src/bin/codegen.rs`:**

```rust
#[allow(unused_imports)]
use your_crate_with_amethystate_types as _;

amethystate_codegen::amethystate_codegen_main!(
    rs_out = "../src/bindings/amethystate.rs",
    framework = leptos
);
```

**3. Run:**

```sh
cargo run --bin codegen
```

The bindings call each struct by its own name. Of a line kept beside its older versions for a migration, only the newest is written. Two different structs of one name are refused, with both their modules named, rather than one of them picked.

## Examples

- [`tauri-typescript`](https://github.com/guinea-rs/amethystate/tree/master/examples/tauri-typescript) — TypeScript frontend
- [`tauri-leptos`](https://github.com/guinea-rs/amethystate/tree/master/examples/tauri-leptos) — Leptos WASM frontend
- [`tauri-yew`](https://github.com/guinea-rs/amethystate/tree/master/examples/tauri-yew) — Yew WASM frontend