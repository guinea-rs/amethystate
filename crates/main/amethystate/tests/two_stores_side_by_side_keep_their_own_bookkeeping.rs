#![cfg(feature = "json")]

use amethystate::store::StoreLayout;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{AmeData, amethystate, migrate};
use amethystate_core::test_utils::TempPath;
use std::path::{Path, PathBuf};

mod v1 {
    use super::*;

    #[amethystate(prefix = "ui", version = 1)]
    pub struct Ui {
        #[amestate(default = 0u32)]
        pub hits: u32,
    }
}

#[amethystate(prefix = "ui", version = 2)]
pub struct Ui {
    #[amestate(default = 0u32)]
    pub hits: u32,
}

#[migrate]
fn double_the_hits(old: AmeData<v1::Ui>) -> amethystate::MigrationResult<AmeData<Ui>> {
    Ok(AmeData::<Ui> { hits: old.hits * 2 })
}

fn written_at_v1(at: &Path, hits: u32) {
    let store = StoreBuilder::new(at)
        .backend(Backend::Json)
        .build()
        .unwrap();
    v1::Ui::new_with(&store).unwrap().hits().set(hits).unwrap();
    store.save_now().unwrap();
}

fn hits_after_migrating(at: &Path) -> u32 {
    let (store, _) = StoreBuilder::new(at)
        .backend(Backend::Json)
        .migrate()
        .unwrap();
    store.get::<u32>(["ui", "hits"]).unwrap().unwrap()
}

fn bookkeeping_of(at: &Path) -> PathBuf {
    match StoreLayout::of(at, Backend::Json) {
        StoreLayout::Sidecars { meta, .. } => meta,
        other => panic!("a text store keeps its bookkeeping beside it, not {other:?}"),
    }
}

#[test]
fn two_stores_that_differ_after_the_last_dot_migrate_each_its_own_data() {
    let dir = TempPath::new("side_by_side");
    std::fs::create_dir_all(dir.path()).unwrap();
    let user = dir.path().join("settings.user");
    let system = dir.path().join("settings.system");

    written_at_v1(&user, 21);
    written_at_v1(&system, 5);

    assert_eq!(hits_after_migrating(&user), 42);
    assert_eq!(hits_after_migrating(&system), 10);
}

#[test]
fn a_store_whose_bookkeeping_sits_under_the_former_name_migrates_once() {
    let dir = TempPath::new("former_name");
    std::fs::create_dir_all(dir.path()).unwrap();
    let at = dir.path().join("settings.json");
    let former = dir.path().join("settings.meta");

    written_at_v1(&at, 21);
    std::fs::rename(bookkeeping_of(&at), &former).unwrap();

    assert_eq!(hits_after_migrating(&at), 42);
    assert_eq!(hits_after_migrating(&at), 42);
    assert!(bookkeeping_of(&at).exists());
    assert!(former.exists());
}

#[test]
fn a_file_under_the_former_name_that_does_not_read_is_not_the_bookkeeping() {
    let dir = TempPath::new("former_name_unreadable");
    std::fs::create_dir_all(dir.path()).unwrap();
    let at = dir.path().join("settings.json");
    let former = dir.path().join("settings.meta");
    {
        let store = StoreBuilder::new(&at)
            .backend(Backend::Json)
            .build()
            .unwrap();
        Ui::new_with(&store).unwrap().hits().set(21).unwrap();
        store.save_now().unwrap();
    }
    std::fs::remove_file(bookkeeping_of(&at)).unwrap();
    std::fs::write(&former, "[editor]\nfont = 'mono'\n").unwrap();

    let hits = StoreBuilder::new(&at)
        .backend(Backend::Json)
        .build()
        .map(|store| Ui::new_with(&store).unwrap().hits().get())
        .map_err(|why| why.to_string());

    assert_eq!(
        (hits, std::fs::read_to_string(&former).unwrap()),
        (Ok(21), "[editor]\nfont = 'mono'\n".to_string())
    );
}

#[test]
fn the_bookkeeping_is_named_after_the_whole_data_file() {
    assert_eq!(
        bookkeeping_of(Path::new("app/settings.json")),
        Path::new("app/settings.json.meta")
    );
}
