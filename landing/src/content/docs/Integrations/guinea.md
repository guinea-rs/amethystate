---
title: guinea
---

`amethystate-guinea` connects a [guinea](https://crates.io/crates/guinea) timer
to a reactive setting. Changing the setting restarts the timer's wait with
the new interval, whether the change came from UI code, another thread or
a file edit.

## Installation

```toml
[dependencies]
amethystate-guinea = "0.29"
```

The adapter uses amethystate's version number and requires Rust 1.95.
It requires guinea 0.32. For guinea 0.31, use adapter 0.28; for guinea 0.30,
use adapter 0.27. The adapter does not enable guinea's default features, so your
application chooses its GUI backend and runtime.

## A period that follows a field

```rust
use amethystate_guinea::IntoChanging;
use guinea::sources::Period;
use std::time::Duration;

cx.every(
    Period::follows(settings.ping_interval_ms().changing(), Duration::from_millis),
    || Ping,
    agent.clone(),
);
```

`changing()` adapts a field for `Period::follows`. The timer reads its value
before each wait and subscribes to changes. A change interrupts the current
wait; the next tick occurs after the new interval. The scope owns the timer,
and leaving the scope ends the timer and its subscription. Dropping the
returned `Source` handle does not stop it.

The field may change on any thread; guinea handles re-arming on its UI thread.

## A period that follows a map entry

`ReactiveCell` also supports `changing()`, so a timer can follow one map entry:

```rust
let ping = intervals.entry_cell("ping".to_string());

cx.every(Period::follows(ping.changing(), Duration::from_millis), || Ping, agent.clone());
```

An absent entry has no interval, so the timer pauses. Inserting the key starts
it; removing the entry or dropping its source map pauses it again.
