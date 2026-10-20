use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{Store, StoreBackend, StoreEvent, SubscriptionKind};
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;
use std::sync::{Arc, Mutex};

mod as_text {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u32, into: S) -> Result<S::Ok, S::Error> {
        into.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(from: D) -> Result<u32, D::Error> {
        String::deserialize(from)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[amethystate(prefix = "replaced")]
pub struct Replaced {
    #[amestate(default = 0u32)]
    pub plain: u32,

    #[amestate(with = as_text, default = 0u32)]
    pub spelled: u32,
}

fn events(store: &Store) -> Arc<Mutex<Vec<StoreEvent>>> {
    let seen: Arc<Mutex<Vec<StoreEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    StoreBackend::subscribe(
        store,
        SubscriptionKind::Any,
        Arc::new(move |event| {
            sink.lock().unwrap().push(event.clone());
            Ok(())
        }),
    );
    seen
}

#[backends(all)]
fn a_field_write_after_a_flush_carries_the_value_it_replaced(backend: Backend) {
    let path = TempPath::new("field_replaced");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();
    let replaced = Replaced::new_with(&store);
    replaced.plain().set(10);
    replaced.spelled().set(10);
    store.save_now().unwrap();
    let seen = events(&store);

    replaced.plain().set(20);
    replaced.spelled().set(20);

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "{backend:?}: {seen:?}");
    assert_eq!(
        store.decode::<u32>(seen[0].old.as_ref().unwrap()).unwrap(),
        10,
        "{backend:?}"
    );
    assert_eq!(
        store
            .decode::<String>(seen[1].old.as_ref().unwrap())
            .unwrap(),
        "10",
        "{backend:?}"
    );
}

#[backends(all)]
fn a_field_write_of_the_value_already_saved_says_nothing(backend: Backend) {
    let path = TempPath::new("field_unchanged");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();
    let replaced = Replaced::new_with(&store);
    replaced.plain().set(10);
    replaced.spelled().set(10);
    store.save_now().unwrap();
    let seen = events(&store);

    replaced.plain().set(10);
    replaced.spelled().set(10);

    assert!(seen.lock().unwrap().is_empty(), "{backend:?}");
}
