---
title: Migrating from a custom solution
sidebar:
  order: 24
---

If an existing TOML, JSON or RON file stores the struct's fields at the top
level, you may be able to adopt it directly. Other layouts need an explicit import.

## The file is your struct

Start with the struct currently loaded and saved through serde:

<!-- shown: the struct serde was writing -->
```rust
#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub window: Window,
}
```
<!-- /shown -->

Declare matching fields at the store root:

<!-- shown: the same fields at the top of the store -->
```rust
#[amethystate(mode = "persistent", as_root)]
pub struct Settings {
    #[amestate(default = "light".to_string())]
    pub theme: String,

    #[amestate(default = Window { width: 800, height: 600 })]
    pub window: Window,
}
```
<!-- /shown -->

Enable the matching storage engine:

```toml
amethystate = { version = "0.29", features = ["toml"] }
```

Open the existing file:

<!-- shown: opening the file that was there -->
```rust
let store = StoreBuilder::new(file).backend(Backend::Toml).build()?;
let settings = Settings::load_with(&store);
```
<!-- /shown -->

`as_root` preserves the fields' top-level locations. `load_with` reads existing
values and uses defaults for missing fields. The store adds a `.meta` file
alongside the data; see [Files](/amethystate/store/files/).

For compatible TOML and JSON layouts, the old struct can still deserialize the
saved file. Check RON separately: the engine writes a map, `{ "theme": .. }`,
whereas a derived RON struct normally expects `(theme: ..)`.

If the old declaration used serde field renames, map them to the appropriate
state attributes; see [Serde](/amethystate/state/serde/).

## Everything else

For a custom database, another format or a different layout, import the values
in application code. Open the new store, read the old source with its existing
reader, save the values through the new state struct, then remove the old source:

<!-- shown: moving the values in once -->
```rust
let mut settings = Settings::load_with(&store);

if let Some(old) = read_old_settings(&old_file)? {
    settings.mutate(|now| {
        now.theme = old.theme;
        now.window = old.window;
    })?;
    std::fs::remove_file(&old_file)?;
}
```
<!-- /shown -->

`mutate` waits for persistence. Delete the old file only after it succeeds.
If the process stops before deletion, both sources remain and the next run
repeats the import, so the conversion must tolerate repetition.

This initial import is not a store migration step: a fresh empty store skips
historical steps. See
[Clean installs](/amethystate/migrations/overview/#what-happens-on-a-clean-install).
