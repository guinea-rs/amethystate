mod common;

use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;

#[amethystate(prefix = "watched")]
pub struct Watched {
    #[amestate(default = 800u32)]
    pub width: u32,
}

#[amethystate(prefix = "held", mode = "persistent")]
pub struct Held {
    #[amestate(default = 800u32)]
    pub width: u32,

    #[amestate(default = "dark".to_string())]
    pub theme: String,
}

#[backends(all)]
fn a_persistent_struct_loads_what_is_there_and_defaults_the_rest(backend: Backend) {
    let path = TempPath::new("shapes_persistent_load");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    store.set(["held", "width"], &1920u32).unwrap();
    store.save_now().unwrap();

    let held = Held::load_with(&store).unwrap();

    assert_eq!(held.width, 1920, "what was stored");
    assert_eq!(
        held.theme, "dark",
        "and the declared default for what was not"
    );
}

#[backends(all)]
fn a_persistent_struct_writes_through_mutate(backend: Backend) {
    let path = TempPath::new("shapes_persistent_save");

    {
        let store = StoreBuilder::new(path.path())
            .backend(backend)
            .build()
            .unwrap();
        let mut held = Held::load_with(&store).unwrap();

        held.mutate(|it| it.width = 1280).unwrap();

        assert_eq!(
            store.get::<u32>(["held", "width"]).unwrap(),
            Some(1280),
            "`mutate` saves, so the store has it before anything is closed"
        );
        store.close().unwrap();
    }

    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    assert_eq!(Held::load_with(&store).unwrap().width, 1280);
}

#[backends(all)]
fn a_persistent_struct_reads_and_writes_through_deref(backend: Backend) {
    let path = TempPath::new("shapes_persistent_deref");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    let mut held = Held::load_with(&store).unwrap();

    held.theme = "light".to_string();
    held.save().unwrap();

    assert_eq!(
        store.get::<String>(["held", "theme"]).unwrap().as_deref(),
        Some("light"),
        "`DerefMut` reaches the data, and `save` puts it where it goes"
    );
}

#[backends(all)]
fn the_watching_shape_is_still_what_it_was(backend: Backend) {
    let path = TempPath::new("shapes_reactive");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    let watched = Watched::new_with(&store).unwrap();
    watched.width.set(1024).unwrap();
    store.save_now().unwrap();

    assert_eq!(store.get::<u32>(["watched", "width"]).unwrap(), Some(1024));
}
