#![cfg(all(
    any(feature = "json", feature = "toml", feature = "ron"),
    not(target_arch = "wasm32")
))]

use amethystate::store::WhenItWillNotRead;
use amethystate::store::builder::StoreBuilder;
use amethystate_core::test_utils::TempPath;
use std::path::Path;

mod common;
use common::text_backend;

fn saved(path: &Path, width: u32) {
    let store = StoreBuilder::new(path)
        .backend(text_backend())
        .build()
        .unwrap();
    store.set(["cfg", "width"], &width).unwrap();
    store.save_now().unwrap();
}

fn width_at(path: &Path) -> Option<u32> {
    StoreBuilder::new(path)
        .backend(text_backend())
        .build()
        .unwrap()
        .get::<u32>(["cfg", "width"])
        .unwrap()
}

#[cfg(unix)]
fn link(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn link(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).unwrap();
}

#[test]
fn a_store_behind_a_link_writes_through_it() {
    let dotfiles = TempPath::new("behind_a_link_target");
    let at = TempPath::new("behind_a_link");
    saved(dotfiles.path(), 800);
    link(dotfiles.path(), at.path());

    saved(at.path(), 1280);

    assert!(
        std::fs::symlink_metadata(at.path())
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(width_at(dotfiles.path()), Some(1280));
}

#[test]
fn a_file_behind_a_link_that_will_not_read_is_set_aside_beside_itself() {
    let dotfiles = TempPath::new("set_aside_behind_a_link_target");
    let at = TempPath::new("set_aside_behind_a_link");
    saved(dotfiles.path(), 800);
    link(dotfiles.path(), at.path());

    let store = StoreBuilder::new(at.path())
        .backend(text_backend())
        .when_it_will_not_read(WhenItWillNotRead::SetAside)
        .build()
        .unwrap();
    std::fs::write(dotfiles.path(), "half-written {{{").unwrap();
    store.set(["cfg", "width"], &1280u32).unwrap();
    store.save_now().unwrap();
    drop(store);

    let mut aside = dotfiles.path().as_os_str().to_os_string();
    aside.push(".unreadable");
    let width = StoreBuilder::new(dotfiles.path())
        .backend(text_backend())
        .build()
        .ok()
        .and_then(|store| store.get::<u32>(["cfg", "width"]).ok().flatten());
    assert_eq!(
        (
            std::fs::symlink_metadata(at.path())
                .unwrap()
                .file_type()
                .is_symlink(),
            width,
            std::fs::read_to_string(&aside).ok(),
        ),
        (true, Some(1280), Some("half-written {{{".to_string()))
    );
}

#[cfg(unix)]
#[test]
fn a_replaced_file_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let path = TempPath::new("keeps_its_permissions");
    saved(path.path(), 800);
    std::fs::set_permissions(path.path(), std::fs::Permissions::from_mode(0o644)).unwrap();

    saved(path.path(), 1280);

    let mode = std::fs::metadata(path.path()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o644);
}
