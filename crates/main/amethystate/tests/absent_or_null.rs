//! Tags: option, toml, json, ron, choosing an engine
//!
//! Features: json, toml, ron
//!
//! A missing key and a stored `None` are different states. This test shows
//! whether the text engines preserve that distinction.
//!
//! JSON writes a present optional value directly and uses `null` for `None`.
//! RON keeps the `Option` representation, so hand-edited values use `Some(...)`
//! or `None`.
//!
//! TOML has no null value. Setting the field to `None` removes its key from
//! the document, producing the same representation as deleting it.
//! The output below shows both the saved document and the result of reading
//! the value back through the store.

use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder, default_backend};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

mod common;

#[amethystate(prefix = "maybe")]
pub struct Held {
    #[amestate(default = None)]
    pub note: Option<String>,
}

fn note_path() -> StorePath {
    StorePath::from_segments(["maybe", "note"])
}

fn holds_nothing(backend: Backend) -> Option<Option<String>> {
    if backend.extension() == "toml" {
        None
    } else {
        Some(None)
    }
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn what_a_document_holds_for_nothing() -> anyhow::Result<()> {
    for backend in common::text_backends() {
        let path = TempPath::new(&format!("absent_or_null_{}", backend.extension()));
        let store = StoreBuilder::new(path.path()).backend(backend).build()?;

        //@act
        let held = Held::new_with(&store);

        //@show a value
        held.note().durable().set(Some("here".to_string()))?;
        let with_value = std::fs::read_to_string(path.path())?;
        //@show-end

        //@show the same key set to nothing
        held.note().durable().set(None)?;
        let with_nothing = std::fs::read_to_string(path.path())?;
        //@show-end

        let read: Option<Option<String>> = store.get(note_path())?;
        //@end

        common::measured(&[
            ("engine", backend.extension()),
            ("get::<Option<String>>", &format!("{read:?}")),
            ("a value", with_value.replace("\r\n", "\n").trim()),
            (
                "the same key set to nothing",
                with_nothing.replace("\r\n", "\n").trim(),
            ),
            ("lang", backend.extension()),
        ]);

        assert_ne!(
            with_value,
            with_nothing,
            "on {}: a value and its absence must not write the same document",
            backend.extension()
        );
        assert_eq!(
            read,
            holds_nothing(backend),
            "on {}: what the engine reads back for a value set to nothing",
            backend.extension()
        );
    }

    Ok(())
}

#[test]
fn nothing_and_gone_are_different() -> anyhow::Result<()> {
    let path = TempPath::new("absent_or_gone");
    let store = StoreBuilder::new(path.path()).build()?;

    let held = Held::new_with(&store);
    held.note().set(None);

    assert_eq!(
        store.get::<Option<String>>(note_path())?,
        holds_nothing(default_backend()),
        "set to nothing"
    );

    store.delete(note_path())?;

    assert_eq!(
        store.get::<Option<String>>(note_path())?,
        None,
        "deleted: there is no key"
    );

    Ok(())
}
