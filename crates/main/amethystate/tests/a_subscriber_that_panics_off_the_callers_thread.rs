#![cfg(all(
    any(feature = "json", feature = "toml", feature = "ron"),
    not(target_arch = "wasm32")
))]

use amethystate::store::builder::StoreBuilder;
use amethystate::{StoreBackend, SubscriptionKind};
use amethystate_core::test_utils::TempPath;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod common;
use common::text_backend;

#[test]
fn a_subscriber_that_panics_on_an_edit_a_save_took_in_leaves_the_store_writable() {
    let path = TempPath::new("panics_on_a_save");
    let store = StoreBuilder::new(path.path())
        .backend(text_backend())
        .disk(|d| d.debounce(Duration::from_secs(60)))
        .build()
        .unwrap();
    store.set(["cfg", "width"], &1280u32).unwrap();
    store.save_now().unwrap();

    store.set(["cfg", "height"], &720u32).unwrap();
    let data = StoreBackend::files_layout(&store).unwrap().names()[0].clone();
    let on_disk = std::fs::read_to_string(&data).unwrap();
    std::fs::write(&data, on_disk.replace("1280", "1920")).unwrap();

    let once = Arc::new(AtomicBool::new(true));
    let not_yet = once.clone();
    let _listening = store.subscribe(
        SubscriptionKind::Any,
        Arc::new(move |_event| {
            if once.swap(false, Ordering::AcqRel) {
                panic!("a subscriber with a bug in it");
            }
            Ok(())
        }),
    );
    let _ = store.save_now();
    assert!(!not_yet.load(Ordering::Acquire));

    store.set(["cfg", "depth"], &32u32).unwrap();
    store.save_now().unwrap();
    assert_eq!(store.get::<u32>(["cfg", "width"]).unwrap(), Some(1920));
}
