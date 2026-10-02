#![cfg(not(target_arch = "wasm32"))]

use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{ReactiveMap, SubscriptionKind, amethystate};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[amethystate(prefix = "load")]
pub struct Load {
    #[amestate(default = {})]
    pub cores: ReactiveMap<String, u64>,
}

#[backends(all)]
fn a_write_that_landed_while_another_was_finishing_is_the_one_the_map_holds(backend: Backend) {
    let path = TempPath::new("map_two_threads");
    let store = StoreBuilder::new(&path).backend(backend).build().unwrap();
    let cores = Load::new_with(&store).unwrap().cores();

    let once = Arc::new(AtomicBool::new(true));
    let other = cores.clone();
    let _meanwhile = store.subscribe(
        SubscriptionKind::Prefix(StorePath::from_segments(["load", "cores"])),
        Arc::new(move |_event| {
            if once.swap(false, Ordering::AcqRel) {
                let other = other.clone();
                std::thread::spawn(move || other.insert("cpu".to_string(), &2).unwrap())
                    .join()
                    .unwrap();
            }
            Ok(())
        }),
    );

    cores.insert("cpu".to_string(), &1).unwrap();

    assert_eq!(
        store
            .get::<u64>(StorePath::from_segments(["load", "cores", "cpu"]))
            .unwrap(),
        Some(2),
        "{backend:?}"
    );
    assert_eq!(cores.get("cpu"), Some(2), "{backend:?}");
}
