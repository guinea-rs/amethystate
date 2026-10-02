---
title: GPUI
---


GPUI uses an entity model with deferred notification — mutations happen inside entity update closures, and the framework notifies dependents after the closure returns. Synchronous `Field<T>` subscriptions don't compose with this model directly, so the integration bridges them through an async channel.

## How it works

`amethystate-gpui` provides `AmeView<T>` — a wrapper that holds a state struct and a `ReactiveScope`. On construction it subscribes to all external changes on the struct and sends a unit message over an unbounded channel. A background task inside the entity drains that channel and calls `entity_cx.notify()`, which triggers a GPUI re-render.

This means GPUI reads state synchronously during `render` via `.get()`, while change detection happens asynchronously in the background.

## Setup

```toml
[dependencies]
amethystate-gpui = "*"
```

Initialize the store before opening any windows:

```rust
StoreBuilder::new("./app.redb").init_global();
```

## Defining state

```rust
#[amethystate(prefix = "counter")]
pub struct CounterState {
    #[amestate(default = 0)]
    pub count: i32,
}
```

## Creating an entity

Use `cx.new_amethystate()` instead of `cx.new()` to wrap a state struct in an `AmeEntity`:

```rust
struct CounterView {
    state: AmeEntity<CounterState>,
}

impl CounterView {
    fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new_amethystate(CounterState::new);
        Self { state }
    }
}
```

`new_amethystate` panics with the error when the struct fails to open. `try_new_amethystate` takes the same closure and returns the error instead.

`AmeEntity<T>` is an alias for `Entity<AmeView<T>>`. `AmeView` derefs to `T`, so state fields are accessed directly through the entity.

## Reading state in render

```rust
impl Render for CounterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_count = self.state.read(cx).count().get();

        div().child(format!("Count: {}", current_count))
    }
}
```

## Writing state

Writes can happen from anywhere — the entity's `on_click` handler, a background thread, another part of the app. The entity subscribes to its state with `external`, so what notifies it is a write somebody else made: a write through a fork triggers a `notify()` and a re-render on its own. A write through the entity's own handle carries the entity's own id and is skipped, so the handler that made it calls `notify()` itself:

```rust
// from a click handler inside render
let state = self.state.clone();
Button::new("Increment")
    .on_click(move |_, _, cx| {
        state.read(cx).count().update(|v| v + 1).ok();
        state.update(cx, |_, cx| cx.notify());
    })

// from a background thread via fork
let forked = state.read(cx).fork();
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_secs(2));
        forked.count().update(|v| v + 1).ok();
    }
});
```

A background thread has no `cx` to notify with, so it writes through a fork, and the entity hears those writes by itself.

## Which GPUI

The adapter builds on [`gpui-pre`](https://crates.io/crates/gpui-pre) 0.3.6, a crates.io release of GPUI, and needs Rust 1.95. Depend on the same package under the name `gpui`, so the app and the adapter share one copy of the crate:

```toml
[dependencies]
gpui = { package = "gpui-pre", version = "0.3.6" }
```

Two copies, say the adapter's and one from git, are two different crates to Cargo, and their types do not match.

## Examples

- [`gpui`](https://github.com/guinea-rs/amethystate/tree/master/examples/gpui)