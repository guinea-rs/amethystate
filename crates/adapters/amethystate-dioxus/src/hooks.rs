use crate::MapSignal;
use amethystate::{MapChange, ReactiveMapKey, ReactiveMapValue};
use amethystate_arena::{AmeStateFrameworkNested, DefaultArena, FieldHandle, MapHandle};
use dioxus::core::{Callback, spawn, use_hook};
use dioxus::hooks::{try_use_context, use_callback, use_context};
use dioxus::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

pub type Handle<S> = <S as AmeStateFrameworkNested>::Handle;

#[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
pub fn use_amethystate<S>() -> S::Handle
where
    S: AmeStateFrameworkNested + 'static,
{
    if let Some(handle) = try_use_context::<S::Handle>() {
        return handle;
    }

    panic!(
        "amethystate-dioxus: State slice '{}' was not initialized! \
         Make sure to include it in preload_slices!(...) at the root AmeStateProvider.",
        std::any::type_name::<S>()
    );
}

#[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
pub fn use_amethystate<S>() -> S::Handle
where
    S: amethystate_arena::AmeStateFramework<crate::DioxusBackend> + 'static,
{
    if let Some(handle) = try_use_context::<S::Handle>() {
        return handle;
    }

    let store = try_use_context::<amethystate::Store>().unwrap_or_else(|| {
        panic!(
            "amethystate-dioxus: Store not found in context while trying to initialize '{}'. \
             Make sure AmeStateProvider is rendered at the root of your application.",
            std::any::type_name::<S>()
        );
    });
    let arena = try_use_context::<DefaultArena>().unwrap_or_else(|| {
        panic!(
            "amethystate-dioxus: DefaultArena not found in context while trying to initialize '{}'. \
             Make sure AmeStateProvider is rendered at the root of your application.",
            std::any::type_name::<S>()
        );
    });

    let handle = use_hook(|| {
        arena
            .slice(|arena| S::load_slice(&store).map(|state| state.register(arena)))
            .unwrap_or_else(|err| {
                panic!(
                    "amethystate-dioxus: Failed to load state slice '{}': {err}",
                    std::any::type_name::<S>()
                );
            })
    });

    use_context_provider(|| handle);
    handle
}

type Tell<V> = Box<dyn Fn(V) + Send + Sync>;

/// Holds what `watch` returned for `at`, and watches again when `at` changes.
fn use_rewatch<A, S>(at: A, watch: impl FnOnce(&A) -> S) -> bool
where
    A: Clone + PartialEq + 'static,
    S: 'static,
{
    let watched = use_hook(|| Rc::new(RefCell::new(None::<(A, S)>)));
    let mut watched = watched.borrow_mut();
    if matches!(&*watched, Some((was, _)) if *was == at) {
        return false;
    }
    let renewed = watched.take().is_some();
    let held = watch(&at);
    *watched = Some((at, held));
    renewed
}

/// The value `read` finds at `at`, kept in step by what `watch` tells it,
/// and read afresh whenever the component is handed another `at`.
///
/// A value told by a watch that has since been replaced is dropped, so a
/// change to what the component looked at before cannot land after the switch.
fn use_watched<A, V, S>(
    at: A,
    read: impl Fn(&A) -> V,
    watch: impl FnOnce(&A, Tell<V>) -> S,
) -> Signal<V>
where
    A: Clone + PartialEq + 'static,
    V: Send + 'static,
    S: 'static,
{
    let mut value = use_signal(|| read(&at));
    let generation = use_hook(|| Arc::new(AtomicU64::new(0)));
    let tx = use_hook(|| {
        let (tx, mut rx) = mpsc::unbounded_channel::<(u64, V)>();
        let current = generation.clone();
        spawn(async move {
            while let Some((told_at, told)) = rx.recv().await {
                if told_at == current.load(Ordering::Acquire) {
                    value.set(told);
                }
            }
        });
        tx
    });

    let mut fresh = None;
    let renewed = use_rewatch(at, |at| {
        let now = generation.fetch_add(1, Ordering::AcqRel) + 1;
        let held = watch(
            at,
            Box::new(move |told| {
                let _ = tx.send((now, told));
            }),
        );
        fresh = Some(read(at));
        held
    });
    if let (true, Some(fresh)) = (renewed, fresh) {
        value.set(fresh);
    }

    value
}

pub fn use_field<T>(handle: FieldHandle<T>) -> (ReadSignal<T>, Callback<T>)
where
    T: DeserializeOwned + Serialize + Clone + Send + Sync + PartialEq + 'static,
{
    let arena = use_context::<DefaultArena>();
    let reading = arena.clone();
    let watching = arena.clone();
    let signal = use_watched(
        handle,
        move |handle| reading.get_field(*handle),
        move |handle, tell| watching.subscribe_field(*handle, move |val: &T| tell(val.clone())),
    );

    let arena_clone = arena.clone();

    let setter = use_callback(move |val: T| {
        #[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
        {
            let arena = arena_clone.clone();
            spawn(async move {
                let _ = arena.set_field(handle, val).await;
            });
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
        {
            let _ = arena_clone.set_field(handle, val);
        }
    });

    (signal.into(), setter)
}

pub fn use_read_only_field<T>(handle: FieldHandle<T>) -> ReadSignal<T>
where
    T: DeserializeOwned + Serialize + Clone + Send + Sync + PartialEq + 'static,
{
    let arena = use_context::<DefaultArena>();
    let reading = arena.clone();
    use_watched(
        handle,
        move |handle| reading.get_field(*handle),
        move |handle, tell| arena.subscribe_field(*handle, move |val: &T| tell(val.clone())),
    )
    .into()
}

pub fn use_map<K, V>(handle: MapHandle<K, V>) -> MapSignal<K, V>
where
    K: ReactiveMapKey + for<'de> Deserialize<'de>,
    V: ReactiveMapValue,
{
    let arena = use_context::<DefaultArena>();
    let reading = arena.clone();
    let watching = arena.clone();
    let signal = use_watched(
        handle,
        move |handle| {
            reading
                .get_map_entries(*handle)
                .into_iter()
                .collect::<HashMap<K, V>>()
        },
        move |handle, tell| {
            let handle = *handle;
            let entries = watching.clone();
            watching.subscribe_map_any(handle, move |_| {
                tell(entries.get_map_entries(handle).into_iter().collect())
            })
        },
    );

    let arena_set = arena.clone();
    let _set = use_callback(move |(key, val): (K, V)| {
        #[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
        {
            let arena = arena_set.clone();
            spawn(async move {
                let _ = arena.set_map_entry(handle, key, val).await;
            });
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
        {
            let _ = arena_set.set_map_entry(handle, key, val);
        }
    });

    let arena_insert = arena.clone();
    let _insert = use_callback(move |(key, val): (K, V)| {
        #[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
        {
            let arena = arena_insert.clone();
            spawn(async move {
                let _ = arena.set_map_entry(handle, key, val).await;
            });
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
        {
            let _ = arena_insert.set_map_entry(handle, key, val);
        }
    });

    let arena_remove = arena.clone();
    let _remove = use_callback(move |key: K| {
        #[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
        {
            let arena = arena_remove.clone();
            spawn(async move {
                let _ = arena.remove_map_entry(handle, key).await;
            });
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
        {
            let _ = arena_remove.remove_map_entry(handle, &key);
        }
    });

    let arena_clear = arena.clone();
    let _clear = use_callback(move |_: ()| {
        #[cfg(all(target_arch = "wasm32", feature = "tauri-backend"))]
        {
            let arena = arena_clear.clone();
            spawn(async move {
                let _ = arena.clear_map(handle).await;
            });
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "tauri-backend")))]
        {
            let _ = arena_clear.clear_map(handle);
        }
    });

    MapSignal::new(signal.into(), _set, _insert, _remove, _clear)
}

pub fn use_map_entry<K, V>(handle: MapHandle<K, V>, key: K) -> ReadSignal<Option<V>>
where
    K: ReactiveMapKey + for<'de> Deserialize<'de>,
    V: ReactiveMapValue,
{
    let arena = use_context::<DefaultArena>();
    let reading = arena.clone();
    use_watched(
        (handle, key),
        move |(handle, key)| reading.get_map_entry(*handle, key),
        move |(handle, key), tell| {
            arena.subscribe_map_key(*handle, key.clone(), move |change| match change {
                MapChange::Insert { value, .. }
                | MapChange::Update {
                    new_value: value, ..
                } => tell(Some(value.clone())),
                MapChange::Remove { .. } | MapChange::Clear { .. } => tell(None),
            })
        },
    )
    .into()
}

pub fn use_map_subscribe_any<K, V, F>(handle: MapHandle<K, V>, callback: F)
where
    K: ReactiveMapKey + for<'de> Deserialize<'de>,
    V: ReactiveMapValue,
    F: Fn(&MapChange<K, V>) + Send + Sync + 'static,
{
    let arena = use_context::<DefaultArena>();
    use_rewatch(handle, move |handle| {
        arena.subscribe_map_any(*handle, callback)
    });
}

pub fn use_map_subscribe_key<K, V, F>(handle: MapHandle<K, V>, key: K, callback: F)
where
    K: ReactiveMapKey + for<'de> Deserialize<'de>,
    V: ReactiveMapValue,
    F: Fn(&MapChange<K, V>) + Send + Sync + 'static,
{
    let arena = use_context::<DefaultArena>();
    use_rewatch((handle, key), move |(handle, key)| {
        arena.subscribe_map_key(*handle, key.clone(), callback)
    });
}
