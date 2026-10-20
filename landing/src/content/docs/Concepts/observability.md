---
title: Observability
sidebar:
  order: 19
---

`amethystate` uses [`tracing`](https://docs.rs/tracing) for structured logs.
Its events use the `amethystate` target, which can be filtered separately
from application logs.

## Enabling trace output

Field and subscription trace events use `TRACE`. Enable that level for the
`amethystate` target in your tracing subscriber:

```rust
tracing_subscriber::fmt()
    .with_env_filter("amethystate=trace")
    .init();
```

You can also set the filter through `RUST_LOG`:

```sh
RUST_LOG=amethystate=trace cargo run
```

## What gets logged

### Field writes

A call to `field.set()` produces a write-attempt trace:

```
TRACE amethystate: field write path=network.port source=NetworkState
TRACE amethystate: field write path=network.port source=external
```

| Field | Meaning |
| --- | --- |
| `path` | The stored path, for example `network.port`. |
| `source` | The short name of the struct owning the writing handle, or `external` for an unattached handle such as a fork or `field_with_path` field. |

### Subscription fires

Each callback invocation produces a subscription trace:

```
TRACE amethystate: signal emit → subscription fire subscription_id=0 name=PortWatcher location=src/main.rs:42
TRACE amethystate: map signal emit → any subscription fire subscription_id=1 name=None location=src/server.rs:88
```

| Field | Meaning |
| --- | --- |
| `subscription_id` | Increasing subscription ID within its signal. |
| `name` | Label set with `.named()`, or `None`. |
| `location` | File and line of the subscription call. |

## Labeling subscriptions

The call site is captured with `#[track_caller]`. Add `.named()` when a generic
helper or framework call site does not identify the subscription clearly:

```rust
let _sub = state.port()
    .subscribe(|p| do_something(p))
    .named("PortWatcher");

// ReactiveMap works the same way
let _sub = state.limits()
    .subscribe_any(|change| handle(change))
    .named("LimitsWatcher");
```

The label appears in that subscription's trace events.

## Reading the output

A write followed by two callbacks looks like this:

```
TRACE amethystate: field write path=network.port source=NetworkState
TRACE amethystate: signal emit → subscription fire subscription_id=0 name=PortWatcher location=src/ui.rs:55
TRACE amethystate: signal emit → subscription fire subscription_id=1 name=None location=src/logger.rs:12
```

`source=external` does **not** mean an external process. It means a local
`set()` through a handle with no owning struct. File edits and migration writes
do not pass through `Field::set`, so they produce no `field write` trace.
