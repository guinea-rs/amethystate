use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use amethystate::store::field_with_path;
use amethystate::test_utils::unique_store;
use amethystate::uuid::Uuid;
use amethystate_guinea::IntoChanging;
use guinea_app::timers::{Changing, Period};

fn counter() -> (Arc<AtomicUsize>, Box<dyn Fn() + Send + Sync>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    (
        calls,
        Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        }),
    )
}

#[test]
fn a_field_reads_what_it_holds_now() {
    let (_at, store) = unique_store("guinea-now");
    let interval = field_with_path(&store, ["net", "ping_ms"], 500u64, Uuid::new_v4()).unwrap();
    let changing = interval.clone().changing();

    assert_eq!(changing.now(), Some(500));

    interval.set(250).unwrap();

    assert_eq!(changing.now(), Some(250));
}

#[test]
fn a_write_to_the_field_calls_whoever_watches_it() {
    let (_at, store) = unique_store("guinea-watch");
    let interval = field_with_path(&store, ["net", "ping_ms"], 500u64, Uuid::new_v4()).unwrap();
    let changing = interval.clone().changing();
    let (calls, changed) = counter();

    let _guard = changing.watch(changed);
    interval.set(250).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_dropped_guard_hears_no_more_writes() {
    let (_at, store) = unique_store("guinea-drop");
    let interval = field_with_path(&store, ["net", "ping_ms"], 500u64, Uuid::new_v4()).unwrap();
    let changing = interval.clone().changing();
    let (calls, changed) = counter();

    let guard = changing.watch(changed);
    interval.set(250).unwrap();
    drop(guard);
    interval.set(100).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn an_absent_entry_has_nothing_to_read_until_it_is_inserted() {
    let (_at, store) = unique_store("guinea-entry");
    let intervals = store.kv().map::<String, u64>("intervals").unwrap();
    let changing = intervals.entry_cell("ping".to_string()).changing();
    let (calls, changed) = counter();
    let _guard = changing.watch(changed);

    assert_eq!(changing.now(), None);

    intervals.insert("ping".to_string(), &750).unwrap();

    assert_eq!(changing.now(), Some(750));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn an_entry_whose_map_is_gone_has_nothing_to_read() {
    let (_at, store) = unique_store("guinea-gone");
    let intervals = store.kv().map::<String, u64>("intervals").unwrap();
    intervals.insert("ping".to_string(), &750).unwrap();
    let changing = intervals.entry_cell("ping".to_string()).changing();

    drop(intervals);

    assert_eq!(changing.now(), None);
}

#[test]
fn a_period_follows_a_field() {
    let (_at, store) = unique_store("guinea-period");
    let interval = field_with_path(&store, ["net", "ping_ms"], 500u64, Uuid::new_v4()).unwrap();

    let period = Period::follows(interval.changing(), Duration::from_millis);

    assert!(matches!(period, Period::Following { .. }));
}
