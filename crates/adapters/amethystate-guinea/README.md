# amethystate-guinea

Lets a [guinea](https://crates.io/crates/guinea) timer's period follow a value
held by an [amethystate](https://crates.io/crates/amethystate) store: change the
setting, and the timer re-arms with the new period at once.

```rust,ignore
use amethystate_guinea::IntoChanging;
use guinea::timers::Period;
use std::time::Duration;

cx.every(
    Period::follows(settings.ping_interval_ms().changing(), Duration::from_millis),
    &agent,
    || Ping,
);
```

`changing()` takes a `Field<T>` or a `ReactiveCell<T>`. A map entry is followed
through `map.entry_cell(key)`: while the key is absent, or once the map is
dropped, there is nothing to read and the timer does not tick.

The crate's version is amethystate's; it builds against guinea 0.18 to 0.23.
