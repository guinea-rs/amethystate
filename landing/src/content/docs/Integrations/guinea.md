---
title: guinea
---

`amethystate-guinea` lets a [guinea](https://crates.io/crates/guinea) timer's
period follow a value in the store. Change the setting, from the UI, from
another thread or by editing the file, and the timer re-arms with the new
period at once, without waiting out the old one.

## Installation

```toml
[dependencies]
amethystate-guinea = "0.24"
```

The crate's version is amethystate's. It builds against guinea 0.18 and every later release, and like guinea it needs Rust 1.95.

## A period that follows a field

```rust
use amethystate_guinea::IntoChanging;
use guinea::timers::Period;
use std::time::Duration;

cx.every(
    Period::follows(settings.ping_interval_ms().changing(), Duration::from_millis),
    &agent,
    || Ping,
);
```

`changing()` turns the field into what `Period::follows` asks for. guinea reads
the value before every wait and watches the field for as long as the timer
runs: a write cuts the wait under way short, and the next tick comes one new
period after it. The watch ends with the timer, or when `Timer::period` gives
it another period.

A write can come from any thread. guinea moves the re-arming to the UI thread
itself.

## A period that follows a map entry

A `ReactiveCell` takes `changing()` too, and that is how a map entry is
followed:

```rust
let ping = intervals.entry_cell("ping".to_string());

cx.every(Period::follows(ping.changing(), Duration::from_millis), &agent, || Ping);
```

While the key is absent the cell has nothing to read, and the timer does not
tick. It starts once the entry is inserted, and stops again if the entry is
removed or the map is dropped.
