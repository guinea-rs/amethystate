#![cfg(all(feature = "test-utils", feature = "json", not(target_arch = "wasm32")))]

use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::store::reactive_map_with_path;
use amethystate::uuid::Uuid;
use amethystate::{ReactiveMap, Store, amethystate};
use amethystate_core::test_utils::TempPath;
use std::collections::HashMap;
use std::path::Path;

fn one() -> HashMap<String, u32> {
    HashMap::from([("one".to_string(), 1)])
}

#[amethystate(prefix = "shipped")]
pub struct Shipped {
    #[amestate(default = one())]
    pub items: ReactiveMap<String, u32>,
}

const CHILD: &str = "AME_STOPPED_SAVE_CHILD";
const STOP: &str = "AMETHYSTATE_STOP_THE_SAVE_AFTER";

fn opened(at: &Path) -> Store {
    StoreBuilder::new(at)
        .backend(Backend::Json)
        .build()
        .unwrap()
}

fn the_childs_store(at: &str) -> Store {
    StoreBuilder::new(Path::new(at))
        .backend(Backend::Json)
        .disk(|d| d.debounce(std::time::Duration::from_secs(60)))
        .build()
        .unwrap()
}

fn in_the_child(change: fn(&Store)) {
    if let Ok(child) = std::env::var(CHILD) {
        let store = the_childs_store(&child);
        change(&store);
        let _ = store.save_now();
        std::process::exit(0);
    }
}

fn stop_the_child(test_name: &str, at: &Path) {
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .env(CHILD, at)
        .env(STOP, "first")
        .output()
        .expect("spawning the save failed");

    assert!(
        !child.status.success(),
        "the save was not stopped: {}",
        String::from_utf8_lossy(&child.stderr)
    );
}

#[test]
fn a_save_that_gave_an_empty_store_its_first_key_and_stopped_opens_again() {
    in_the_child(|store| store.set(["cfg", "width"], &1280u32).unwrap());

    let at = TempPath::new("a_save_stopped_filling");
    opened(at.path()).save_now().unwrap();

    stop_the_child(
        "a_save_that_gave_an_empty_store_its_first_key_and_stopped_opens_again",
        at.path(),
    );

    let store = StoreBuilder::new(at.path()).backend(Backend::Json).build();
    assert!(store.is_ok(), "{:?}", store.err());
}

#[test]
fn a_map_seeded_by_a_save_that_stopped_comes_up_with_its_defaults() {
    in_the_child(|store| {
        Shipped::new_with(store);
    });

    let at = TempPath::new("a_save_stopped_seeding");
    {
        let store = opened(at.path());
        store.set(["cfg", "width"], &1280u32).unwrap();
        store.save_now().unwrap();
    }

    stop_the_child(
        "a_map_seeded_by_a_save_that_stopped_comes_up_with_its_defaults",
        at.path(),
    );

    let store = opened(at.path());
    let shipped = Shipped::new_with(&store);
    assert_eq!(shipped.items().get("one"), Some(1));
}

#[test]
fn a_map_seeded_by_a_process_that_died_before_saving_comes_up_with_its_defaults() {
    if let Ok(child) = std::env::var(CHILD) {
        let store = the_childs_store(&child);
        reactive_map_with_path::<String, u32>(&store, ["items"], one(), Uuid::new_v4()).unwrap();
        std::process::abort();
    }

    let at = TempPath::new("a_process_died_after_seeding");
    {
        let store = opened(at.path());
        store.set(["cfg", "width"], &1280u32).unwrap();
        store.save_now().unwrap();
    }

    stop_the_child(
        "a_map_seeded_by_a_process_that_died_before_saving_comes_up_with_its_defaults",
        at.path(),
    );

    let store = opened(at.path());
    let items =
        reactive_map_with_path::<String, u32>(&store, ["items"], one(), Uuid::new_v4()).unwrap();
    assert_eq!(items.get("one"), Some(1));
}

#[test]
fn a_save_that_emptied_the_store_and_stopped_opens_again() {
    in_the_child(|store| store.delete(["cfg", "width"]).unwrap());

    let at = TempPath::new("a_save_stopped_emptying");
    {
        let store = opened(at.path());
        store.set(["cfg", "width"], &1280u32).unwrap();
        store.save_now().unwrap();
    }

    stop_the_child(
        "a_save_that_emptied_the_store_and_stopped_opens_again",
        at.path(),
    );

    let store = StoreBuilder::new(at.path()).backend(Backend::Json).build();
    assert!(store.is_ok(), "{:?}", store.err());
}
