#![cfg(all(target_arch = "wasm32", feature = "tauri"))]

use amethystate::amethystate;
use amethystate::tauri::TauriBackend;
use amethystate::uuid::Uuid;
use amethystate_core::path::StorePath;
use std::collections::HashMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(inline_js = r#"
export function pretend_tauri() {
    window.__ame_written = [];
    window.__ame_listening = [];
    window.__ame_watched = {};
    window.__ame_held = {};
    window.__TAURI_INTERNALS__ = {
        invoke: async (cmd, args) => {
            if (cmd === "plugin:amethystate|amethystate_set") {
                window.__ame_written.push(args.key);
                window.__ame_held[args.key] = args.value;
            }
            if (cmd === "plugin:amethystate|amethystate_get") {
                return window.__ame_held[args.key] ?? null;
            }
            if (cmd === "plugin:amethystate|amethystate_subscribe") {
                window.__ame_watched[args.key] = (window.__ame_watched[args.key] ?? 0) + 1;
            }
            if (cmd === "plugin:amethystate|amethystate_unsubscribe") {
                window.__ame_watched[args.key] = (window.__ame_watched[args.key] ?? 0) - 1;
            }
            if (cmd === "plugin:event|listen") {
                const id = (window.__ame_next_id = (window.__ame_next_id ?? 0) + 1);
                window.__ame_listening.push({ id, event: args.event, handler: args.handler });
                return id;
            }
            if (cmd === "plugin:event|unlisten") {
                window.__ame_listening = window.__ame_listening.filter(
                    (listener) => listener.id !== args.eventId,
                );
            }
            return null;
        },
        transformCallback: (callback) => callback,
    };
}

export function forget_tauri() {
    window.__TAURI_INTERNALS__ = undefined;
}

export function written() {
    return window.__ame_written.join("\n");
}

export function watched() {
    return Object.entries(window.__ame_watched)
        .map(([key, count]) => `${key} ${count}`)
        .sort()
        .join("\n");
}

export function tell(event, payload) {
    for (const listener of window.__ame_listening) {
        if (listener.event === event) {
            listener.handler({ event, id: 0, payload: JSON.parse(payload) });
        }
    }
}
"#)]
extern "C" {
    fn pretend_tauri();
    fn forget_tauri();
    fn written() -> String;
    fn watched() -> String;
    fn tell(event: &str, payload: &str);
}

async fn settled() {
    for _ in 0..20 {
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
            &wasm_bindgen::JsValue::NULL,
        ))
        .await
        .unwrap();
    }
}

#[amethystate(target = "tauri-wasm")]
pub struct Db {
    #[amestate(default = "localhost".to_string())]
    pub host: String,
}

#[amethystate(prefix = "sys", target = "tauri-wasm")]
pub struct Sys {
    #[amestate(nested, flatten)]
    pub db: Db,

    #[amestate(volatile, default = 0u32)]
    pub clicks: u32,

    #[amestate(default = 0u64)]
    pub last_seen_ns: u64,
}

#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Opaque {
    pub inside: u8,
}

#[amethystate(prefix = "shelf", target = "tauri-wasm")]
pub struct Shelf {
    #[amestate(default = Opaque::default())]
    pub opaque: Opaque,
}

#[amethystate(prefix = "generated", target = "tauri-wasm")]
pub struct Generated {
    #[amestate(default = ::amethystate::serde_json::from_str(r#"7"#).unwrap_or_default())]
    pub retries: u8,
}

#[amethystate(prefix = "visits", target = "tauri-wasm")]
pub struct Visits {
    #[amestate(default = {})]
    pub pages: ReactiveMap<String, u32>,
}

fn stored(pairs: &[(&str, serde_json::Value)]) -> HashMap<StorePath, serde_json::Value> {
    pairs
        .iter()
        .map(|(at, value)| (StorePath::parse_joined(at).unwrap(), value.clone()))
        .collect()
}

#[wasm_bindgen_test]
fn a_flattened_child_reads_its_fields_at_its_parents_level() {
    pretend_tauri();
    let initial = stored(&[("sys.host", serde_json::json!("db.local"))]);

    let sys = Sys::new_with_id(&initial, &TauriBackend, Uuid::new_v4());

    assert_eq!(sys.db.host.path.to_string(), "sys.host");
    assert_eq!(sys.db.host.value(), "db.local");
}

#[wasm_bindgen_test]
fn a_field_the_store_does_not_hold_starts_from_the_default_the_bindings_carry() {
    pretend_tauri();

    let generated = Generated::new_with_id(&HashMap::new(), &TauriBackend, Uuid::new_v4());

    assert_eq!(generated.retries.value(), 7);
}

#[wasm_bindgen_test]
fn a_volatile_field_does_not_read_what_is_stored_at_its_name() {
    pretend_tauri();
    let initial = stored(&[("sys.clicks", serde_json::json!(7))]);

    let sys = Sys::new_with_id(&initial, &TauriBackend, Uuid::new_v4());

    assert_eq!(sys.clicks.value(), 0);
}

#[wasm_bindgen_test]
async fn a_volatile_field_is_set_here_and_sent_nowhere() {
    pretend_tauri();
    let sys = Sys::new_with_id(&HashMap::new(), &TauriBackend, Uuid::new_v4());

    sys.clicks.set(3).await.unwrap();

    assert_eq!(sys.clicks.value(), 3);
    assert_eq!(written(), "");
}

#[wasm_bindgen_test]
fn a_struct_holding_a_value_with_no_debug_still_prints() {
    pretend_tauri();
    let id = Uuid::new_v4();

    let shelf = Shelf::new_with_id(&HashMap::new(), &TauriBackend, id);

    assert_eq!(
        format!("{shelf:?}"),
        format!("Shelf {{ instance_id: {id:?}, .. }}")
    );
}

#[wasm_bindgen_test]
async fn a_struct_let_go_stops_watching_every_path_it_watched() {
    pretend_tauri();
    let sys = Sys::new_with_id(&HashMap::new(), &TauriBackend, Uuid::new_v4());
    settled().await;
    let while_held = watched();

    drop(sys);
    settled().await;

    assert_eq!(
        [while_held, watched()],
        [
            "sys.host 1\nsys.last_seen_ns 1",
            "sys.host 0\nsys.last_seen_ns 0"
        ]
    );
}

#[wasm_bindgen_test]
async fn a_number_javascript_cannot_hold_is_refused_rather_than_crashing_the_page() {
    pretend_tauri();
    let sys = Sys::new_with_id(&HashMap::new(), &TauriBackend, Uuid::new_v4());

    let refused = sys.last_seen_ns.set(u64::MAX).await;

    assert!(refused.is_err());
    assert_eq!(written(), "");
}

#[wasm_bindgen_test]
async fn a_page_outside_tauri_is_told_so_rather_than_crashing() {
    use amethystate::client::AmeBackendAsync;

    forget_tauri();

    let refused = TauriBackend
        .get::<String>(&StorePath::parse_joined("sys.host").unwrap())
        .await
        .unwrap_err();

    assert_eq!(
        refused.current_context(),
        &amethystate::tauri::Error::Command(
            "TypeError: Cannot read properties of undefined (reading 'invoke')".to_string()
        )
    );
}

#[wasm_bindgen_test]
async fn an_entry_is_written_from_what_it_held_or_from_nothing() {
    pretend_tauri();
    let visits = Visits::new_with_id(&HashMap::new(), &TauriBackend, Uuid::new_v4());

    let mut counted = Vec::new();
    for page in ["home", "about", "home"] {
        let now = visits
            .pages
            .upsert(page.to_string(), |seen| seen.map_or(1, |count| count + 1))
            .await
            .unwrap();
        counted.push(now);
    }

    drop(visits);
    settled().await;

    assert_eq!(counted, [1, 1, 2]);
    assert_eq!(
        written(),
        "visits.pages.home\nvisits.pages.about\nvisits.pages.home"
    );
}

#[wasm_bindgen_test]
async fn a_field_takes_what_the_host_tells_it_and_what_it_says_when_the_key_goes() {
    pretend_tauri();
    let initial = stored(&[("sys.host", serde_json::json!("db.local"))]);
    let sys = Sys::new_with_id(&initial, &TauriBackend, Uuid::new_v4());
    settled().await;

    let mut seen = Vec::new();
    for told in [
        r#"{ "type": "Value", "value": "elsewhere", "source": null }"#,
        r#"{ "type": "Deleted", "source": null }"#,
        r#"{ "type": "Deleted", "value": "localhost", "source": null }"#,
    ] {
        tell("amethystate://sys:host", told);
        settled().await;
        seen.push(sys.db.host.value());
    }

    assert_eq!(seen, ["elsewhere", "elsewhere", "localhost"]);
}
