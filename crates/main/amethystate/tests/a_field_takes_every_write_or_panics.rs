use amethystate::amethystate;
use amethystate::store::builder::StoreBuilder;
use amethystate_core::test_utils::TempPath;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing_test::traced_test;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

mod common;

#[amethystate(prefix = "taken")]
pub struct Taken {
    #[amestate(default = 1u8)]
    pub level: u8,

    #[amestate(default = None)]
    pub nested: Option<Option<u32>>,
}

#[cfg(not(target_arch = "wasm32"))]
fn said(panic: Box<dyn std::any::Any + Send>) -> String {
    match panic.downcast::<String>() {
        Ok(said) => *said,
        Err(panic) => panic
            .downcast::<&'static str>()
            .map(|said| said.to_string())
            .unwrap_or_default(),
    }
}

#[traced_test]
#[test]
fn a_write_after_the_close_changes_nothing_and_says_so() {
    for backend in common::enabled_backends() {
        let at = TempPath::new("taken_closed");
        let store = StoreBuilder::new(at.path())
            .backend(backend)
            .build()
            .unwrap();
        let taken = Taken::new_with(&store);
        taken.level().set(7);
        store.close().unwrap();

        taken.level().set(9);

        assert_eq!(taken.level().get(), 7, "{backend:?}");
        assert!(
            logs_contain("taken.level was not written: the store is closed"),
            "{backend:?}"
        );
    }
}

#[cfg(all(feature = "json", not(target_arch = "wasm32")))]
#[test]
fn a_value_the_engine_cannot_hold_panics_naming_the_field() {
    use amethystate::store::builder::Backend;

    let at = TempPath::new("taken_unencodable");
    let store = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .build()
        .unwrap();
    let taken = Taken::new_with(&store);

    let written = catch_unwind(AssertUnwindSafe(|| taken.nested().set(Some(None))));

    let Err(panic) = written else {
        panic!("a value json cannot hold was taken without a word");
    };
    let said = said(panic);
    assert!(said.contains("taken.nested"), "{said}");
    assert!(said.contains("holding nothing"), "{said}");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_struct_over_a_value_that_will_not_read_panics_naming_it() {
    for backend in common::enabled_backends() {
        let at = TempPath::new("taken_unreadable");
        let store = StoreBuilder::new(at.path())
            .backend(backend)
            .build()
            .unwrap();
        store.set(["taken", "level"], &"high".to_string()).unwrap();

        let opened = catch_unwind(AssertUnwindSafe(|| Taken::new_with(&store)));

        let Err(panic) = opened else {
            panic!("{backend:?}: opened over a level that is not a number");
        };
        let said = said(panic);
        assert!(said.contains("Taken"), "{backend:?}: {said}");
        assert!(
            said.contains("taken.level will not read back"),
            "{backend:?}: {said}"
        );
    }
}
