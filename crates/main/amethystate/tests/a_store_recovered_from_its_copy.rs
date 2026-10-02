#![cfg(any(feature = "json", feature = "toml", feature = "ron"))]

use amethystate::store::builder::StoreBuilder;
use amethystate_core::test_utils::TempPath;
use std::path::{Path, PathBuf};

mod common;
use common::text_backend;

fn appended(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn left_with_a_copy_and_a_broken_file(suffix: &str) -> TempPath {
    let path = TempPath::new(suffix);
    {
        let store = StoreBuilder::new(path.path())
            .backend(text_backend())
            .build()
            .unwrap();
        store.set(["cfg", "width"], &1280u32).unwrap();
        store.save_now().unwrap();
    }
    std::fs::copy(path.path(), appended(path.path(), ".bak")).unwrap();
    std::fs::write(path.path(), "not a document {{{").unwrap();
    path
}

#[test]
fn a_recovery_takes_the_copy_away_and_sets_the_unreadable_file_aside() {
    let path = left_with_a_copy_and_a_broken_file("recovered_copy_goes");

    let store = StoreBuilder::new(path.path())
        .backend(text_backend())
        .build()
        .unwrap();
    assert_eq!(store.get::<u32>(["cfg", "width"]).unwrap(), Some(1280));
    drop(store);

    assert!(!appended(path.path(), ".bak").exists());
    assert_eq!(
        std::fs::read_to_string(appended(path.path(), ".unreadable")).unwrap(),
        "not a document {{{"
    );
}

#[test]
fn a_file_broken_again_after_a_recovery_is_refused_rather_than_recovered_again() {
    let path = left_with_a_copy_and_a_broken_file("recovered_copy_not_reused");
    drop(
        StoreBuilder::new(path.path())
            .backend(text_backend())
            .build()
            .unwrap(),
    );

    std::fs::write(path.path(), "edited by hand {{{").unwrap();
    let opened = StoreBuilder::new(path.path())
        .backend(text_backend())
        .build();

    assert!(opened.is_err());
    drop(opened);
    assert_eq!(
        std::fs::read_to_string(path.path()).unwrap(),
        "edited by hand {{{"
    );
}
