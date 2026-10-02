use super::scope::Scope;
use amethystate::store::{OnDelete, StoreBackend, StorePath};
use amethystate::{StoreEvent, StoreOp, StoreSubscription, SubscriptionKind};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Runtime, State, Webview};

pub struct PluginState {
    pub store: amethystate::Store,
    pub scope: Scope,
    pub subscriptions: Mutex<HashMap<String, Watched>>,
}

impl PluginState {
    /// The plugin over `store`, reaching the places the exported structs declare.
    pub fn new(store: amethystate::Store) -> Self {
        Self {
            store,
            scope: Scope::exported(),
            subscriptions: Mutex::default(),
        }
    }

    /// Lets go of every path the webview labelled `label` watched.
    pub fn forget(&self, label: &str) {
        let let_go: Vec<Watched> = match self.subscriptions.lock() {
            Ok(mut subs) => {
                let keys: Vec<String> = subs
                    .iter()
                    .filter(|(_, watched)| watched.lets_go_of(label, usize::MAX))
                    .map(|(key, _)| key.clone())
                    .collect();
                keys.iter().filter_map(|key| subs.remove(key)).collect()
            }
            Err(_) => Vec::new(),
        };
        drop(let_go);
    }
}

type Watchers = Arc<Mutex<HashMap<String, usize>>>;

/// A path the frontend listens to, and how many listeners each webview has
/// left on it.
pub struct Watched {
    _listening: StoreSubscription,
    watchers: Watchers,
}

impl Watched {
    fn watched_by(&self, label: String) {
        if let Ok(mut watchers) = self.watchers.lock() {
            *watchers.entry(label).or_default() += 1;
        }
    }

    fn lets_go_of(&self, label: &str, listeners: usize) -> bool {
        let Ok(mut watchers) = self.watchers.lock() else {
            return false;
        };
        if let Some(left) = watchers.get_mut(label) {
            *left = left.saturating_sub(listeners);
            if *left == 0 {
                watchers.remove(label);
            }
        }
        watchers.is_empty()
    }
}

/// Sends `payload` on `channel` to each webview watching it, and to no other.
fn tell<R: Runtime>(app: &AppHandle<R>, watchers: &Watchers, channel: &str, payload: Value) {
    let labels: Vec<String> = match watchers.lock() {
        Ok(watchers) => watchers.keys().cloned().collect(),
        Err(_) => return,
    };
    for label in labels {
        let _ = app.emit_to(label.as_str(), channel, payload.clone());
    }
}

fn parsed(key: &str) -> Result<StorePath, String> {
    StorePath::parse_joined(key).map_err(|e| e.to_string())
}

fn out_of_scope(key: &str) -> String {
    format!("`{key}` is not a place a struct exported to the frontend declares")
}

fn source_of(event: &StoreEvent) -> Option<uuid::Uuid> {
    match event.source {
        amethystate::reactive::Source::Handle(id) => Some(id),
        _ => None,
    }
}

#[tauri::command]
pub async fn amethystate_get(
    store: State<'_, PluginState>,
    key: String,
) -> Result<Option<Value>, String> {
    let path = parsed(&key)?;
    if !store.scope.holds(&path) {
        return Err(out_of_scope(&key));
    }
    store.store.get(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn amethystate_set(
    store: State<'_, PluginState>,
    key: String,
    value: Value,
    source: Option<uuid::Uuid>,
) -> Result<(), String> {
    let path = parsed(&key)?;
    if !store.scope.holds(&path) {
        return Err(out_of_scope(&key));
    }
    if !store.scope.accepts(&path, &value.to_string()) {
        return Err(format!("`{key}` does not hold a value like {value}"));
    }
    store
        .store
        .set_with_source(&path, &value, source)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn amethystate_get_prefix(
    store: State<'_, PluginState>,
    prefix: String,
) -> Result<HashMap<String, Value>, String> {
    let prefix = parsed(&prefix)?;
    let raw = StoreBackend::scan_prefix(&store.store, &prefix).map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for (path, bytes) in raw {
        if store.scope.holds(&path)
            && let Ok(val) = store.store.decode::<Value>(&bytes)
        {
            map.insert(path.to_string(), val);
        }
    }
    Ok(map)
}

#[tauri::command]
pub async fn amethystate_flush(
    store: State<'_, PluginState>,
    prefix: String,
) -> Result<(), String> {
    let prefix = parsed(&prefix)?;
    StoreBackend::flush_prefix(&store.store, &prefix).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn amethystate_subscribe<R: Runtime>(
    store: State<'_, PluginState>,
    app: AppHandle<R>,
    webview: Webview<R>,
    key: String,
) -> Result<(), String> {
    let path = parsed(&key)?;
    let label = webview.label().to_string();
    let mut subs = store.subscriptions.lock().map_err(|e| e.to_string())?;
    if let Some(watched) = subs.get(&key) {
        watched.watched_by(label);
        return Ok(());
    }

    let channel = amethystate::tauri::event_channel(&path);
    let watchers: Watchers = Arc::new(Mutex::new(HashMap::from([(label, 1)])));
    let listening = if let Some(field) = store.scope.field(&path) {
        let resets = field
            .resets
            .unwrap_or_else(|| store.store.fallbacks().on_delete == OnDelete::UseDefault);
        let default = resets
            .then(field.meta.default)
            .flatten()
            .and_then(|json| serde_json::from_str::<Value>(&json).ok());
        watch_field(&store.store, app, channel, watchers.clone(), path, default)
    } else if store.scope.is_map(&path) {
        watch_map(&store.store, app, channel, watchers.clone(), path)
    } else {
        return Err(out_of_scope(&key));
    };

    subs.insert(
        key,
        Watched {
            _listening: listening,
            watchers,
        },
    );
    Ok(())
}

/// Tells the frontend each value the field takes, and that its key went.
///
/// A key that goes is told as `Deleted`, carrying the default the field
/// falls back to where its declarations or the store say it takes one, and no
/// value where it keeps the last.
fn watch_field<R: Runtime>(
    store: &amethystate::Store,
    app: AppHandle<R>,
    channel: String,
    watchers: Watchers,
    path: StorePath,
    default: Option<Value>,
) -> StoreSubscription {
    let decoding = store.clone();
    store.subscribe(
        SubscriptionKind::ExactPath(path),
        Arc::new(move |event| {
            let source = source_of(event);
            let payload = match (&event.op, &event.new) {
                (StoreOp::Set, Some(bytes)) => match decoding.decode::<Value>(bytes) {
                    Ok(value) => json!({ "type": "Value", "value": value, "source": source }),
                    Err(_) => return Ok(()),
                },
                (StoreOp::Set, None) => return Ok(()),
                (StoreOp::Delete | StoreOp::DeletePrefix, _) => match &default {
                    Some(value) => json!({ "type": "Deleted", "value": value, "source": source }),
                    None => json!({ "type": "Deleted", "source": source }),
                },
            };
            tell(&app, &watchers, &channel, payload);
            Ok(())
        }),
    )
}

/// Tells the frontend each change to the map: an entry put, replaced or
/// taken away, and the whole map cleared.
fn watch_map<R: Runtime>(
    store: &amethystate::Store,
    app: AppHandle<R>,
    channel: String,
    watchers: Watchers,
    map: StorePath,
) -> StoreSubscription {
    let decoding = store.clone();
    store.subscribe(
        SubscriptionKind::Prefix(map.clone()),
        Arc::new(move |event| {
            let source = source_of(event);

            if event.path == map {
                if event.op == StoreOp::DeletePrefix {
                    tell(
                        &app,
                        &watchers,
                        &channel,
                        json!({ "type": "Clear", "source": source }),
                    );
                }
                return Ok(());
            }

            if event.path.parent().as_ref() != Some(&map) {
                return Ok(());
            }
            let Some(key) = event.path.name() else {
                return Ok(());
            };
            let key = key.as_str();
            let decoded = |bytes: &Option<Vec<u8>>| {
                bytes
                    .as_ref()
                    .and_then(|b| decoding.decode::<Value>(b).ok())
            };
            let old = decoded(&event.old);

            let payload = match event.op {
                StoreOp::Set => {
                    let new = decoded(&event.new).unwrap_or(Value::Null);
                    match old {
                        Some(old) => json!({
                            "type": "Update",
                            "key": key,
                            "oldValue": old,
                            "newValue": new,
                            "source": source,
                        }),
                        None => json!({
                            "type": "Insert",
                            "key": key,
                            "value": new,
                            "source": source,
                        }),
                    }
                }
                StoreOp::Delete | StoreOp::DeletePrefix => json!({
                    "type": "Remove",
                    "key": key,
                    "oldValue": old.unwrap_or(Value::Null),
                    "source": source,
                }),
            };
            tell(&app, &watchers, &channel, payload);
            Ok(())
        }),
    )
}

#[tauri::command]
pub async fn amethystate_unsubscribe<R: Runtime>(
    state: State<'_, PluginState>,
    webview: Webview<R>,
    key: String,
) -> Result<(), String> {
    let let_go = {
        let mut subs = state.subscriptions.lock().map_err(|e| e.to_string())?;
        match subs.get(&key) {
            Some(watched) if watched.lets_go_of(webview.label(), 1) => subs.remove(&key),
            _ => None,
        }
    };
    drop(let_go);
    Ok(())
}

#[tauri::command]
pub async fn amethystate_delete(
    store: State<'_, PluginState>,
    key: String,
    source: Option<uuid::Uuid>,
) -> Result<(), String> {
    let path = parsed(&key)?;
    if !store.scope.holds(&path) {
        return Err(out_of_scope(&key));
    }
    store
        .store
        .delete_with_source(&path, source)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn amethystate_delete_prefix(
    store: State<'_, PluginState>,
    prefix: String,
    source: Option<uuid::Uuid>,
) -> Result<(), String> {
    let path = parsed(&prefix)?;
    if !store.scope.clears(&path) {
        return Err(out_of_scope(&prefix));
    }
    StoreBackend::delete_prefix_with_source(&store.store, &path, source).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn amethystate_scan_keys(
    store: State<'_, PluginState>,
    prefix: String,
) -> Result<Vec<String>, String> {
    let prefix = parsed(&prefix)?;
    let keys = StoreBackend::scan_keys(&store.store, &prefix).map_err(|e| e.to_string())?;
    Ok(keys
        .iter()
        .filter(|key| store.scope.holds(key))
        .map(|key| key.to_string())
        .collect())
}
