#![cfg(feature = "json")]

use amethystate::store::OpenStore;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{AmeData, MigrationError, amethystate, migrate};
use amethystate_core::test_utils::TempPath;

mod v1 {
    use super::*;

    #[amethystate(prefix = "ledger", version = 1)]
    pub struct Ledger {
        #[amestate(default = 0u32)]
        pub millis: u32,
    }
}

mod v2 {
    use super::*;

    #[amethystate(prefix = "ledger", version = 2)]
    pub struct Ledger {
        #[amestate(default = 0u32)]
        pub millis: u32,
    }
}

#[amethystate(prefix = "ledger", version = 3)]
pub struct Ledger {
    #[amestate(default = 0u32)]
    pub millis: u32,
    #[amestate(default = 1u32)]
    pub pages: u32,
}

#[migrate]
fn to_seconds(old: AmeData<v1::Ledger>) -> amethystate::MigrationResult<AmeData<v2::Ledger>> {
    Ok(AmeData::<v2::Ledger> {
        millis: old.millis / 1000,
    })
}

#[test]
fn a_line_whose_steps_stop_below_its_declared_version_is_a_gap() {
    let at = TempPath::new("declared_past_its_last_step");
    {
        let store = StoreBuilder::new(at.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        v1::Ledger::new_with(&store).millis().set(5000);
        store.save_now().unwrap();
    }

    let Err(OpenStore::Migrating {
        report: Some(report),
        ..
    }) = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .migrate()
    else {
        panic!("the line stopped at v2 and the store opened as if it were at v3");
    };

    let failures = report.failures().collect::<Vec<_>>();
    assert!(
        matches!(
            failures.as_slice(),
            [only] if matches!(
                only.downcast_ref::<MigrationError>(),
                Some(MigrationError::Gap {
                    reached_version: 2,
                    expected_version: 3,
                    ..
                })
            )
        ),
        "{failures:?}"
    );
}
