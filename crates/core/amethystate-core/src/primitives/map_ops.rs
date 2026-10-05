use crate::AmeBackendSync;
use crate::facts::Facts;
use crate::path::StorePath;
use crate::primitives::error::{ReactiveMapError, ReactiveMapResult, WriteValue};
use crate::primitives::map_core::{MapEntryPath, ReactiveMapKey, ReactiveMapValue};
use crate::{MapChange, ReactiveMapCore};
use serde::de::DeserializeOwned;
use uuid::Uuid;

/// Writes a key that already exists, and fails with
/// [`ReactiveMapError::Absent`] otherwise.
pub fn map_update<B, K, V>(
    backend: &B,
    path: StorePath,
    key: K,
    value: &V,
    source: Option<Uuid>,
) -> ReactiveMapResult<()>
where
    B: AmeBackendSync,
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    let full_path = path.entry(key.as_ref());
    let old_value = match read_entry::<B, V>(backend, &full_path)? {
        Some(old_value) => old_value,
        None => return Err(ReactiveMapError::Absent { at: full_path }),
    };

    let change = MapChange::Update {
        key,
        old_value: Some(old_value),
        new_value: value.clone(),
        source,
    };

    map_apply_change(backend, path, change)
}

/// Writes a key whether or not it exists, emitting [`MapChange::Insert`] for
/// a new one and [`MapChange::Update`] for one that was already there.
pub fn map_insert<B, K, V>(
    backend: &B,
    path: StorePath,
    key: K,
    value: &V,
    source: Option<Uuid>,
) -> ReactiveMapResult<()>
where
    B: AmeBackendSync,
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    let full_path = path.entry(key.as_ref());
    let old_value = read_entry::<B, V>(backend, &full_path)?;
    let change = if let Some(old_value) = old_value {
        MapChange::Update {
            key,
            old_value: Some(old_value),
            new_value: value.clone(),
            source,
        }
    } else {
        MapChange::Insert {
            key,
            value: value.clone(),
            source,
        }
    };

    map_apply_change(backend, path, change)
}

pub fn map_remove<B, K, V>(
    backend: &B,
    core: &ReactiveMapCore<K, V>,
    path: StorePath,
    key: K,
    source: Option<Uuid>,
) -> ReactiveMapResult<Option<V>>
where
    B: AmeBackendSync,
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    let exists = core.cache.contains_key(key.as_ref());
    if !exists {
        return Ok(None);
    }

    let full_path = path.entry(key.as_ref());
    let old_value = read_entry::<B, V>(backend, &full_path)?;
    if let Some(old_value) = old_value {
        let change = MapChange::Remove {
            key,
            old_value: Some(old_value.clone()),
            source,
        };
        map_apply_change(backend, path, change)?;
        Ok(Some(old_value))
    } else {
        core.cache.remove(key.as_ref());
        Ok(None)
    }
}

fn read_entry<B, V>(backend: &B, entry: &StorePath) -> ReactiveMapResult<Option<V>>
where
    B: AmeBackendSync,
    V: DeserializeOwned,
{
    backend
        .get::<V>(entry)
        .attach_key(entry)
        .map_err(|why| WriteValue::from_store(entry, why))
}

pub fn map_clear<B, K, V>(
    backend: &B,
    path: StorePath,
    source: Option<Uuid>,
) -> ReactiveMapResult<()>
where
    B: AmeBackendSync,
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    map_apply_change(backend, path, MapChange::<K, V>::Clear { source })
}

/// Writes a change to the backend.
///
/// The key cache is not touched here. The backend tells the map about every
/// write through its subscription, in the order the backend settled them, and
/// that is where the cache follows it. Applying the change again on the way
/// out would put this write back over one another thread landed in between.
pub fn map_apply_change<B, K, V>(
    backend: &B,
    path: StorePath,
    processed: MapChange<K, V>,
) -> ReactiveMapResult<()>
where
    B: AmeBackendSync,
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    match &processed {
        MapChange::Insert { key, value, .. }
        | MapChange::Update {
            key,
            new_value: value,
            ..
        } => {
            let entry = path.entry(key.as_ref());
            backend
                .set_with_source(&entry, value, processed.source())
                .attach_key(&entry)
                .map_err(|why| WriteValue::from_store(&entry, why))?;
        }
        MapChange::Remove { key, .. } => {
            let entry = path.entry(key.as_ref());
            backend
                .delete_with_source(&entry, processed.source())
                .attach_key(&entry)
                .map_err(|why| WriteValue::from_store(&entry, why))?;
        }
        MapChange::Clear { .. } => {
            backend
                .delete_prefix(&path, processed.source())
                .attach_prefix(&path)
                .map_err(|why| WriteValue::from_store(&path, why))?;
        }
    }

    Ok(())
}

/// Brings the key cache in line with a change, whether it was made here or
/// read back from an edit to the file.
///
/// The cache and the store have to agree about which keys exist, and when they
/// drift the failure is silent rather than loud: `remove` gates on the cache,
/// so a key the cache has lost answers `Ok(None)` and deletes nothing, on a
/// key that is really in the store.
pub fn map_apply_remote_change<K, V>(core: &ReactiveMapCore<K, V>, change: &MapChange<K, V>)
where
    K: ReactiveMapKey,
    V: ReactiveMapValue,
{
    let keys = &core.cache;
    match change {
        MapChange::Insert { key, value, .. }
        | MapChange::Update {
            key,
            new_value: value,
            ..
        } => {
            keys.insert(key.clone(), value.clone());
        }
        MapChange::Remove { key, .. } => {
            keys.remove(key.as_ref());
        }
        MapChange::Clear { .. } => {
            keys.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_cache_holds_what_a_change_from_elsewhere_left() {
        let core = ReactiveMapCore::<String, u64>::new();

        map_apply_remote_change(
            &core,
            &MapChange::Insert {
                key: "left".to_string(),
                value: 1,
                source: None,
            },
        );
        map_apply_remote_change(
            &core,
            &MapChange::Insert {
                key: "right".to_string(),
                value: 2,
                source: None,
            },
        );
        assert_eq!(core.cache.get("left"), Some(1));
        assert_eq!(core.cache.get("right"), Some(2));

        map_apply_remote_change(
            &core,
            &MapChange::Update {
                key: "left".to_string(),
                old_value: Some(1),
                new_value: 3,
                source: None,
            },
        );
        assert_eq!(core.cache.get("left"), Some(3));

        map_apply_remote_change(
            &core,
            &MapChange::Remove {
                key: "left".to_string(),
                old_value: Some(3),
                source: None,
            },
        );
        assert!(!core.cache.contains_key("left"));
        assert!(core.cache.contains_key("right"));

        map_apply_remote_change(&core, &MapChange::Clear { source: None });
        assert!(core.cache.is_empty());
    }

    #[test]
    fn a_change_told_after_a_later_one_does_not_replace_it() {
        let cache = crate::primitives::map_core::MapCache::<String, u64>::new();

        cache.insert_settled("cpu".to_string(), 2, 2);
        cache.insert_settled("cpu".to_string(), 1, 1);
        assert_eq!(cache.get("cpu"), Some(2));

        cache.remove_settled("cpu", 4);
        cache.insert_settled("cpu".to_string(), 3, 3);
        assert_eq!(cache.get("cpu"), None);

        cache.insert_settled("gpu".to_string(), 6, 6);
        cache.clear_settled(5);
        assert_eq!(cache.get("gpu"), Some(6));

        cache.insert_settled("npu".to_string(), 4, 4);
        assert_eq!(cache.get("npu"), None);
    }
}
