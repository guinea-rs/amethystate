# amethystate-guinea

Connect a [guinea](https://crates.io/crates/guinea) timer to a reactive
[amethystate](https://crates.io/crates/amethystate) setting. Changing the value
restarts the timer's wait with the new interval.

```rust,ignore
use amethystate_guinea::IntoChanging;
use guinea::sources::Period;
use std::time::Duration;

cx.every(
    Period::follows(settings.ping_interval_ms().changing(), Duration::from_millis),
    || Ping,
    agent.clone(),
);
```

Call `changing()` on a `Field<T>` or `ReactiveCell<T>`. To follow a map entry,
use `map.entry_cell(key)`. An absent entry has no interval, so the timer pauses
until the key is inserted. Dropping the source map also pauses the timer.

The adapter's version follows amethystate. It requires guinea >=0.32 and Rust 1.95.
The guinea dependency has no upper bound, so a new guinea minor release does not
by itself require an adapter release. Breaking changes to the timer API may still
require an adapter update. This release is tested with guinea 0.32, 0.33 and 0.34.
For guinea 0.31, use amethystate-guinea 0.28; for guinea 0.30, use adapter 0.27.
The guinea dependency has default features disabled: the adapter does not select
a GUI backend or start its own runtime.
