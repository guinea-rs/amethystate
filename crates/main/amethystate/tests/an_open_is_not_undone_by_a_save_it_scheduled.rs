#![cfg(feature = "json")]

use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{AmeData, amethystate, migrate};
use amethystate_core::test_utils::TempPath;
use std::time::Duration;

mod common;

mod v1 {
    use super::*;

    #[amethystate(prefix = "first", version = 1)]
    pub struct First {
        #[amestate(default = 0u32)]
        pub hits: u32,
    }

    #[amethystate(prefix = "second", version = 1)]
    pub struct Second {
        #[amestate(default = 0u32)]
        pub n: u32,
    }
}

#[amethystate(prefix = "first", version = 2)]
pub struct First {
    #[amestate(default = 0u32)]
    pub hits: u32,
}

#[amethystate(prefix = "second", version = 2)]
pub struct Second {
    #[amestate(default = 0u32)]
    pub n: u32,
}

#[migrate]
fn double_the_hits(old: AmeData<v1::First>) -> amethystate::MigrationResult<AmeData<First>> {
    Ok(AmeData::<First> { hits: old.hits * 2 })
}

#[migrate]
fn take_a_while(old: AmeData<v1::Second>) -> amethystate::MigrationResult<AmeData<Second>> {
    std::thread::sleep(Duration::from_millis(200));
    Ok(AmeData::<Second> { n: old.n })
}

#[test]
fn a_migrated_line_is_not_put_back_by_a_save_the_open_scheduled() {
    let at = TempPath::new("undone_by_its_save");
    {
        let store = StoreBuilder::new(at.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        v1::First::new_with(&store).hits().set(21);
        v1::Second::new_with(&store).n().set(1);
        store.save_now().unwrap();
    }

    let meta = common::bookkeeping_of(at.path());
    let mut written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    written.as_object_mut().unwrap().remove("format");
    std::fs::write(&meta, written.to_string()).unwrap();

    let (store, _) = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .disk(|d| d.debounce(Duration::ZERO))
        .migrate()
        .unwrap();

    assert_eq!(store.get::<u32>(["first", "hits"]).unwrap(), Some(42));
}
