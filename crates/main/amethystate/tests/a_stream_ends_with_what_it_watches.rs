#![cfg(not(target_arch = "wasm32"))]

use amethystate::store::builder::StoreBuilder;
use amethystate::store::field_with_path;
use amethystate::{ReactiveMap, amethystate};
use amethystate_core::test_utils::TempPath;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

#[amethystate(prefix = "streams")]
pub struct Streams {
    #[amestate(default = {})]
    pub peers: ReactiveMap<String, u32>,
}

#[derive(Default)]
struct Woken(AtomicUsize);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn polled<S: Stream + Unpin>(stream: &mut S, waker: &Waker) -> Poll<Option<S::Item>> {
    Pin::new(stream).poll_next(&mut Context::from_waker(waker))
}

#[test]
fn a_stream_over_a_field_ends_once_every_handle_is_gone() {
    let path = TempPath::new("stream_ends_field");
    let store = StoreBuilder::new(path.path()).build().unwrap();
    let port = field_with_path::<u16>(&store, ["net", "port"], 8080, uuid::Uuid::new_v4()).unwrap();
    let mut changes = port.subscription_with().stream();

    let woken = Arc::new(Woken::default());
    let waker = Waker::from(woken.clone());
    port.set(9090);
    assert_eq!(polled(&mut changes, &waker), Poll::Ready(Some(9090)));
    assert_eq!(polled(&mut changes, &waker), Poll::Pending);

    drop(port);

    assert_eq!(woken.0.load(Ordering::SeqCst), 1);
    assert_eq!(polled(&mut changes, &waker), Poll::Ready(None));
}

#[test]
fn a_stream_over_a_map_ends_once_every_handle_is_gone() {
    let path = TempPath::new("stream_ends_map");
    let store = StoreBuilder::new(path.path()).build().unwrap();
    let peers = Streams::new_with(&store).peers();
    let mut changes = peers.subscription_with().stream();

    let woken = Arc::new(Woken::default());
    let waker = Waker::from(woken.clone());
    assert!(polled(&mut changes, &waker).is_pending());

    drop(peers);

    assert_eq!(woken.0.load(Ordering::SeqCst), 1);
    assert!(matches!(polled(&mut changes, &waker), Poll::Ready(None)));
}
