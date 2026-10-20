---
title: Subscriptions
sidebar:
  order: 17
---

Fields, cells and maps use the same subscription model. This page covers
subscription lifetime, callback threads and identifying the writer.

## A subscription ends with its guard

<!-- shown: subscribing, and letting the subscription go -->
```rust
let sub = state.port().subscribe(move |port| {
    seen.lock().unwrap().push(*port);
});

state.port().set(9090);
assert_eq!(*heard.lock().unwrap(), [9090]);

drop(sub);

state.port().set(1234);
assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

`subscribe` returns a guard that owns the subscription. Keep it while you need
notifications; dropping it unregisters the callback. There is no separate
`unsubscribe` call.

## Keeping the guard is the caller's job

Do not assign the guard to `_`: that drops it at the end of the statement,
before any later changes can reach the callback. A named binding such as `_sub`
keeps it alive:

<!-- shown: a subscription nobody held on to -->
```rust
let ignored = Arc::clone(&heard);
let _ = state.port().subscribe(move |port| {
    ignored.lock().unwrap().push(*port);
});

state.port().set(4321);
assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

`heard` still contains only `[9090]` from the first subscription. Store the guard
alongside the component or state that uses the notifications, so their lifetimes
match.

### Several at once

<!-- shown: keeping several subscriptions in one place -->
```rust
let mut scope = ReactiveScope::new();

state
    .port()
    .subscribe(|port| println!("port {port}"))
    .watch(&mut scope);
state
    .host()
    .subscribe(|host| println!("host {host}"))
    .watch(&mut scope);

scope.clear();
```
<!-- /shown -->

`ReactiveScope` owns several guards. `clear` or dropping the scope ends all of
its subscriptions.

## When plain `subscribe` is not enough

Use `subscription_with()` for additional options:

| method | behavior |
| --- | --- |
| `.external()` | Filter this handle's own value updates. |
| `.key(k)` | Watch one map entry. |
| `.register(f)` | Register a callback and return its guard. |
| `.register_with_source(f)` | Include the writing handle's ID in the callback. |
| `.stream()` | Receive changes as a `Stream`. |

## Taking the changes into a loop of your own

Callbacks must be `Send + Sync`: file-watcher updates can invoke them on a
background thread. That excludes `Rc` state and many UI handles.

With `.stream()`, values are delivered to a stream instead. Process it on the
thread that owns your UI state:

<!-- shown: taking the changes into a loop of your own -->
```rust
let mut ports = state.port().subscription_with().stream();

state.port().set(9090);
state.port().set(1234);

let mut heard = Vec::new();
futures::executor::block_on(async {
    while let Some(port) = ports.next().await {
        heard.push(port);
        if port == 1234 {
            break;
        }
    }
});

assert_eq!(heard, [9090, 1234]);
```
<!-- /shown -->

The stream preserves individual changes rather than coalescing them.
Coalesce them in your consumer if appropriate. Dropping the stream cancels
the subscription.

## Whose write was it

A write includes the writer's handle ID. `.external()` uses it to filter
writes made through the subscribing handle:

<!-- shown: hearing only about somebody else's writes -->
```rust
let watcher = state.port().fork();

let _sub = state
    .port()
    .subscription_with()
    .external()
    .register(move |port| {
        seen.lock().unwrap().push(*port);
    });

state.port().set(8080);
watcher.set(9090);

assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

For example, give a worker a forked handle and subscribe in the UI with
`.external()`. The UI sees the worker's changes without receiving its own
value writes again.

An external file edit has no handle ID, so external-only subscriptions receive
it too.

### `clone` and `fork`

Both handles refer to the same value, but use different writer identities:

<!-- shown: the same actor, and a different one -->
```rust
let port = state.port();
let same = port.clone();
let other = port.fork();

let _sub = port
    .subscription_with()
    .external()
    .register(move |value| seen.lock().unwrap().push(*value));

same.set(1111);
other.set(2222);

assert_eq!(*heard.lock().unwrap(), [2222]);

assert_eq!(port.instance_id(), same.instance_id());
assert_ne!(port.instance_id(), other.instance_id());
```
<!-- /shown -->

`clone` retains the ID: the original and clone count as one writer for
`.external()`. `fork` gets a new ID, so the two handles see each other's writes.

Use `instance_id()` on a field or map to obtain this ID for comparison or
logging.

### The author arrives with the value

<!-- shown: asking who made the change -->
```rust
let _sub = state
    .port()
    .subscription_with()
    .register_with_source(move |port, who| {
        seen.lock().unwrap().push((*port, who));
    });

state.port().set(9090);

let (port, who) = heard.lock().unwrap()[0];
assert_eq!(port, 9090);
assert_eq!(who, Some(state.port().instance_id()));
```
<!-- /shown -->

`register_with_source` passes the writer ID with the value. Use it when the
callback needs to decide separately for each event.

## On a map

On maps, `.external()` filters only `Update`. Structural changes — `Insert`,
`Remove` and `Clear` — reach every subscriber. See
[ReactiveMap](/amethystate/primitives/reactive-map/#what-external-filters)
for the effect on `insert`.
