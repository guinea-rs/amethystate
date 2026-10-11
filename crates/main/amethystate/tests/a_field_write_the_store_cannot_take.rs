use amethystate::SubscriptionKind;
use amethystate::amethystate;
use amethystate::store::StorageError;
use amethystate::store::builder::{Backend, StoreBuilder};
#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
use amethystate::store::{OnUnreadable, OpenStruct};
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;
use std::sync::Arc;

#[amethystate(prefix = "net")]
pub struct Net {
    #[amestate(default = 8080u16)]
    pub port: u16,
}

#[cfg(not(target_arch = "wasm32"))]
fn taken(write: impl FnOnce()) -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(write)).map_err(|panic| {
        match panic.downcast::<String>() {
            Ok(said) => *said,
            Err(panic) => panic
                .downcast::<&'static str>()
                .map(|said| said.to_string())
                .unwrap_or_default(),
        }
    })
}

#[cfg(target_arch = "wasm32")]
fn taken(write: impl FnOnce()) -> Result<(), String> {
    write();
    Ok(())
}

#[backends(all)]
fn a_subscriber_that_refuses_leaves_the_write_standing(backend: Backend) {
    let at = TempPath::new("refusing_subscriber");
    let store = StoreBuilder::new(at.path())
        .backend(backend)
        .build()
        .unwrap();
    let net = Net::new_with(&store);
    let _refusing = store.subscribe(
        SubscriptionKind::Any,
        Arc::new(|_| Err(error_stack::Report::new(StorageError::Notify))),
    );

    let written = taken(|| net.port().set(9));

    assert_eq!(written, Ok(()), "{backend:?}");
    assert_eq!(net.port().get(), 9, "{backend:?}");
    assert_eq!(
        store.get::<u16>(["net", "port"]).unwrap(),
        Some(9),
        "{backend:?}"
    );
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
fn a_value_where_the_level_was(backend: Backend) -> &'static str {
    match backend {
        #[cfg(feature = "json")]
        Backend::Json => r#"{"net": 5}"#,
        #[cfg(feature = "toml")]
        Backend::Toml => "net = 5\n",
        #[cfg(feature = "ron")]
        Backend::Ron => "{\n  \"net\": 5,\n}",
        other => panic!("{other:?} keeps no file a person could edit"),
    }
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[backends(text)]
fn a_level_edited_into_a_value_refuses_the_struct(backend: Backend) {
    let at = TempPath::new("level_edited_refused");
    std::fs::write(at.path(), a_value_where_the_level_was(backend)).unwrap();
    let store = StoreBuilder::new(at.path())
        .backend(backend)
        .build()
        .unwrap();

    let opened = Net::try_new_with(&store);

    assert!(
        matches!(opened, Err(OpenStruct::WillNotRead { .. })),
        "{backend:?}: {:?}",
        opened.err()
    );
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[backends(text)]
fn a_level_edited_into_a_value_takes_the_next_write(backend: Backend) {
    let at = TempPath::new("level_edited_away");
    std::fs::write(at.path(), a_value_where_the_level_was(backend)).unwrap();
    let store = StoreBuilder::new(at.path())
        .backend(backend)
        .rules(|rules| rules.on_unreadable(OnUnreadable::UseDefault))
        .build()
        .unwrap();
    let net = Net::new_with(&store);

    let written = taken(|| net.port().set(1));

    assert_eq!(written, Ok(()), "{backend:?}");
    assert_eq!(net.port().get(), 1, "{backend:?}");
    assert_eq!(
        store.get::<u16>(["net", "port"]).unwrap(),
        Some(1),
        "{backend:?}"
    );
}

#[cfg(all(windows, feature = "json"))]
mod a_disk_that_will_not_take_writes {
    use super::*;
    use amethystate::store::config::WriteAttempts;
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{Duration, Instant};

    fn failing_soon(at: &TempPath) -> amethystate::Store {
        StoreBuilder::new(at.path())
            .backend(Backend::Json)
            .disk(|disk| {
                disk.debounce(Duration::from_millis(5))
                    .retry_every(Duration::from_millis(5))
                    .give_up_after(Duration::ZERO)
            })
            .file_write(|write| {
                write
                    .writing(WriteAttempts::times(1).apart(Duration::from_millis(1)))
                    .replacing(WriteAttempts::times(1).apart(Duration::from_millis(1)))
            })
            .build()
            .unwrap()
    }

    fn held_shut(at: &TempPath) -> std::fs::File {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(at.path())
            .unwrap()
    }

    fn given_up(store: &amethystate::Store) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while store.persist_failure().is_none() {
            assert!(Instant::now() < deadline, "the flush never gave up");
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn a_store_that_gave_up_goes_on_in_memory() {
        let at = TempPath::new("gave_up_in_memory");
        let store = failing_soon(&at);
        let net = Net::new_with(&store);
        store.save_now().unwrap();
        let held = held_shut(&at);

        net.port().set(1);
        given_up(&store);
        let written = taken(|| net.port().set(2));

        assert_eq!(written, Ok(()));
        assert_eq!(net.port().get(), 2);
        assert_eq!(store.get::<u16>(["net", "port"]).unwrap(), Some(2));
        assert!(store.persist_failure().is_some());

        drop(held);
        store.save_now().unwrap();
        let on_disk = std::fs::read_to_string(at.path()).unwrap();
        assert!(on_disk.contains('2'), "{on_disk}");
    }

    #[test]
    fn an_observer_that_panics_does_not_take_the_writes_down() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let at = TempPath::new("panicking_observer");
        let asked = Arc::new(AtomicBool::new(false));
        let told = asked.clone();
        let store = failing_soon(&at);
        let _watch = store.on_persist_failure(move |_| {
            told.store(true, Ordering::SeqCst);
            panic!("the application's own observer")
        });
        let net = Net::new_with(&store);
        store.save_now().unwrap();
        let held = held_shut(&at);

        net.port().set(1);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !asked.load(Ordering::SeqCst) {
            assert!(Instant::now() < deadline, "the flush never gave up");
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(50));
        let written = taken(|| net.port().set(2));
        drop(held);

        assert_eq!(written, Ok(()));
        assert_eq!(net.port().get(), 2);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "json"))]
#[test]
fn a_subscriber_that_panics_leaves_the_write_on_its_way_to_disk() {
    use std::time::{Duration, Instant};

    let at = TempPath::new("panicking_subscriber");
    let store = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .disk(|disk| disk.debounce(Duration::from_millis(5)))
        .build()
        .unwrap();
    let panicking = Net::new_with(&store);
    let other = Net::new_with(&store);
    store.save_now().unwrap();
    let _sub = panicking.port().subscribe(|port| {
        if *port == 7 {
            panic!("the subscriber's own bug");
        }
    });

    let _ = taken(|| panicking.port().set(7));

    assert_eq!(other.port().get(), 7);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let on_disk = std::fs::read_to_string(at.path()).unwrap();
        if on_disk.contains('7') {
            break;
        }
        assert!(Instant::now() < deadline, "never flushed: {on_disk}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
