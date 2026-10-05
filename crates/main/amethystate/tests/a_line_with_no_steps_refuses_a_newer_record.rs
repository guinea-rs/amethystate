#![cfg(feature = "json")]

use amethystate::store::OpenStore;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{MigrationError, amethystate};
use amethystate_core::test_utils::TempPath;

mod common;

#[amethystate(prefix = "clock", version = 1)]
pub struct Clock {
    #[amestate(default = 0u32)]
    pub timeout: u32,
}

#[test]
fn a_line_with_no_steps_refuses_a_store_a_newer_release_recorded() {
    let at = TempPath::new("no_steps_newer_record");
    {
        let store = StoreBuilder::new(at.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        Clock::new_with(&store).timeout().set(30);
        store.save_now().unwrap();
    }

    let meta = common::bookkeeping_of(at.path());
    let mut written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    written.as_object_mut().unwrap().insert(
        "meta.clock".into(),
        serde_json::json!({ "versions": { "": 2 } }),
    );
    std::fs::write(&meta, written.to_string()).unwrap();

    let Err(OpenStore::Migrating {
        report: Some(report),
        ..
    }) = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .migrate()
    else {
        panic!("a release that declares v1 opened what a release at v2 recorded");
    };

    let failures = report.failures().collect::<Vec<_>>();
    assert!(
        matches!(
            failures.as_slice(),
            [only] if matches!(
                only.downcast_ref::<MigrationError>(),
                Some(MigrationError::Downgrade {
                    db_version: 2,
                    code_version: 1,
                    ..
                })
            )
        ),
        "{failures:?}"
    );
}
