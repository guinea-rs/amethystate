#![cfg(not(target_arch = "wasm32"))]

use amethystate::store::StorePath;
use amethystate::test_utils::unique_store;
use amethystate::{ReactiveMap, StoreBackend, amethystate};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_amethystate::backend::commands::{
    PluginState, amethystate_delete, amethystate_delete_prefix, amethystate_get,
    amethystate_get_prefix, amethystate_scan_keys, amethystate_set, amethystate_subscribe,
};

#[amethystate(prefix = "shown")]
pub struct Shown {
    #[amestate(default = 8080u16, on_delete = UseDefault)]
    pub port: u16,

    #[amestate(default = 3u8)]
    pub retries: u8,

    #[amestate(default = {})]
    pub items: ReactiveMap<String, u32>,

    #[amestate(default = "in memory".to_string(), volatile)]
    pub session: String,
}

fn app_over(store: &amethystate::Store) -> App<MockRuntime> {
    let app = tauri::test::mock_app();
    app.manage(PluginState::new(store.clone()));
    app
}

fn heard_on(app: &App<MockRuntime>, channel: &str) -> Arc<Mutex<Vec<Value>>> {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let sink = heard.clone();
    app.listen_any(channel, move |event| {
        sink.lock()
            .unwrap()
            .push(serde_json::from_str::<Value>(event.payload()).unwrap())
    });
    heard
}

fn settle() {
    std::thread::sleep(Duration::from_millis(200));
}

async fn watch(app: &App<MockRuntime>, key: &str) -> Result<(), String> {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .build()
            .unwrap(),
    };
    amethystate_subscribe(
        app.state::<PluginState>(),
        app.handle().clone(),
        window.as_ref().clone(),
        key.into(),
    )
    .await
}

#[tokio::test]
async fn a_path_no_struct_exported_is_refused() {
    let (_at, store) = unique_store("not_exported");
    store.set(["hidden", "token"], &"secret").unwrap();
    let app = app_over(&store);
    let state = app.state::<PluginState>();

    let refused = [
        amethystate_get(state.clone(), "hidden.token".into())
            .await
            .map(|_| ()),
        amethystate_set(state.clone(), "hidden.token".into(), json!("mine"), None).await,
        amethystate_delete(state.clone(), "hidden.token".into(), None).await,
        watch(&app, "hidden").await,
    ];

    assert!(
        refused.iter().all(Result::is_err),
        "every command reached it: {refused:?}"
    );
    assert_eq!(
        store.get::<String>(["hidden", "token"]).unwrap().as_deref(),
        Some("secret")
    );
}

#[tokio::test]
async fn the_root_is_not_a_prefix_the_frontend_may_clear() {
    let (_at, store) = unique_store("root_not_cleared");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let state = app.state::<PluginState>();

    let cleared = amethystate_delete_prefix(state.clone(), String::new(), None).await;

    assert!(cleared.is_err());
    assert_eq!(store.get::<u16>(["shown", "port"]).unwrap(), Some(8080));
}

#[tokio::test]
async fn a_scan_of_the_root_holds_only_exported_paths() {
    let (_at, store) = unique_store("root_scanned");
    let _shown = Shown::new_with(&store).unwrap();
    store.set(["shown", "items", "a"], &1u32).unwrap();
    store.set(["hidden", "token"], &"secret").unwrap();
    let app = app_over(&store);
    let state = app.state::<PluginState>();

    let mut scanned: Vec<String> = amethystate_get_prefix(state.clone(), String::new())
        .await
        .unwrap()
        .into_keys()
        .collect();
    scanned.sort();
    let mut keys = amethystate_scan_keys(state.clone(), String::new())
        .await
        .unwrap();
    keys.sort();

    assert_eq!(scanned, ["shown.items.a", "shown.port", "shown.retries"]);
    assert_eq!(keys, ["shown.items.a", "shown.port", "shown.retries"]);
}

#[tokio::test]
async fn a_value_of_the_wrong_type_is_refused() {
    let (_at, store) = unique_store("wrong_type");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let state = app.state::<PluginState>();

    let written = [
        amethystate_set(state.clone(), "shown.port".into(), json!("a port"), None).await,
        amethystate_set(state.clone(), "shown.items.a".into(), json!(-1), None).await,
        amethystate_set(state.clone(), "shown.port".into(), json!(9), None).await,
        amethystate_set(state.clone(), "shown.items.a".into(), json!(1), None).await,
    ];

    assert_eq!(
        written.map(|written| written.is_ok()),
        [false, false, true, true]
    );
    assert_eq!(store.get::<u16>(["shown", "port"]).unwrap(), Some(9));
}

#[tokio::test]
async fn a_volatile_field_is_not_reachable() {
    let (_at, store) = unique_store("volatile_unreached");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let state = app.state::<PluginState>();

    let written = amethystate_set(state.clone(), "shown.session".into(), json!("x"), None).await;

    assert!(written.is_err());
    assert_eq!(store.get::<String>(["shown", "session"]).unwrap(), None);
}

#[tokio::test]
async fn a_field_the_store_loses_takes_its_default_on_the_frontend() {
    let (_at, store) = unique_store("field_lost");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let heard = heard_on(&app, "amethystate://shown:port");
    watch(&app, "shown.port").await.unwrap();
    let port = StorePath::from_segments(["shown", "port"]);

    store.set(["shown", "port"], &9u16).unwrap();
    settle();
    store.delete(&port).unwrap();
    settle();
    store.set(["shown", "port"], &10u16).unwrap();
    settle();
    store.delete_prefix_with_source(&port, None).unwrap();
    settle();

    assert_eq!(
        *heard.lock().unwrap(),
        [
            json!({ "type": "Value", "value": 9, "source": null }),
            json!({ "type": "Deleted", "value": 8080, "source": null }),
            json!({ "type": "Value", "value": 10, "source": null }),
            json!({ "type": "Deleted", "value": 8080, "source": null }),
        ]
    );
}

#[tokio::test]
async fn a_window_that_watches_nothing_hears_nothing() {
    let (_at, store) = unique_store("window_unwatched");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let other = WebviewWindowBuilder::new(&app, "other", WebviewUrl::default())
        .build()
        .unwrap();
    let heard_elsewhere = Arc::new(Mutex::new(Vec::new()));
    let sink = heard_elsewhere.clone();
    other.listen("amethystate://shown:port", move |event| {
        sink.lock().unwrap().push(event.payload().to_string())
    });
    let heard = heard_on(&app, "amethystate://shown:port");
    watch(&app, "shown.port").await.unwrap();

    store.set(["shown", "port"], &9u16).unwrap();
    settle();

    assert_eq!(
        (
            heard.lock().unwrap().len(),
            heard_elsewhere.lock().unwrap().len()
        ),
        (1, 0)
    );
}

#[tokio::test]
async fn a_field_that_keeps_its_value_is_told_only_that_the_key_went() {
    let (_at, store) = unique_store("field_kept");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let heard = heard_on(&app, "amethystate://shown:retries");
    watch(&app, "shown.retries").await.unwrap();

    store.delete(["shown", "retries"]).unwrap();
    settle();

    assert_eq!(
        *heard.lock().unwrap(),
        [json!({ "type": "Deleted", "source": null })]
    );
}

#[tokio::test]
async fn an_entry_cleared_by_its_prefix_leaves_the_rest_of_the_map() {
    let (_at, store) = unique_store("entry_cleared");
    let _shown = Shown::new_with(&store).unwrap();
    let app = app_over(&store);
    let heard = heard_on(&app, "amethystate://shown:items");
    watch(&app, "shown.items").await.unwrap();

    store.set(["shown", "items", "a"], &1u32).unwrap();
    settle();
    store
        .delete_prefix_with_source(&StorePath::from_segments(["shown", "items", "a"]), None)
        .unwrap();
    settle();
    store
        .delete_prefix_with_source(&StorePath::from_segments(["shown", "items"]), None)
        .unwrap();
    settle();

    assert_eq!(
        *heard.lock().unwrap(),
        [
            json!({ "type": "Insert", "key": "a", "value": 1, "source": null }),
            json!({ "type": "Remove", "key": "a", "oldValue": null, "source": null }),
            json!({ "type": "Clear", "source": null }),
        ]
    );
}
