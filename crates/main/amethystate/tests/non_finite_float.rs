//! Tags: codec, json, sqlite, choosing an engine
//!
//! Features: redb, sqlite, json, toml, ron
//!
//! This test checks which engines preserve non-finite `f64` values:
//! `NaN`, positive infinity and negative infinity.
//!
//! redb's MessagePack codec preserves their floating-point representation.
//! TOML and RON also support non-finite numbers.
//!
//! JSON has no representation for them. The JSON serializers used here would
//! write `null`, which cannot be read back as `f64`. This affects SQLite too,
//! because the SQLite engine encodes values as JSON.
//!
//! amethystate rejects these lossy writes before buffering them. A durable
//! write returns `Err`; an ordinary field `set` panics and identifies the field.
//!
//! `limits(|l| l.portable_across(..))` also checks target engines that are not
//! currently running. For example, redb with JSON portability enabled rejects
//! a NaN even though MessagePack itself could preserve it.

use amethystate::amethystate;
#[cfg(any(feature = "json", feature = "sqlite"))]
use amethystate::errors::WriteValue;
#[cfg(feature = "json")]
use amethystate::store::builder::Backend;
use amethystate::store::builder::StoreBuilder;
#[cfg(any(
    feature = "redb",
    feature = "sqlite",
    feature = "json",
    feature = "toml",
    feature = "ron"
))]
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

mod common;

#[amethystate(prefix = "nonfinite")]
pub struct Readings {
    #[amestate(default = 0.0f64)]
    pub ratio: f64,
}

#[cfg(any(
    feature = "redb",
    feature = "sqlite",
    feature = "json",
    feature = "toml",
    feature = "ron"
))]
fn ratio_path() -> StorePath {
    StorePath::from_segments(["nonfinite", "ratio"])
}

#[cfg(any(feature = "redb", feature = "toml", feature = "ron"))]
#[test]
fn a_format_that_can_hold_it_keeps_it() {
    let backend = common::enabled_backends()
        .into_iter()
        .find(|b| matches!(common::engine_name(*b), "redb" | "toml" | "ron"))
        .expect("msgpack writes the bits, and toml and ron have the words");

    let path = TempPath::new("nonfinite_ok");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    let state = Readings::new_with(&store);
    state.ratio().set(f64::NAN);

    assert!(
        state.ratio().get().is_nan(),
        "the handle should still hold what was written"
    );

    let read: Option<f64> = store
        .get(ratio_path())
        .expect("a typed read should not fail on a value the format can hold");
    assert!(
        read.map(f64::is_nan).unwrap_or(false),
        "read back: {read:?}"
    );
}

#[cfg(any(feature = "json", feature = "sqlite"))]
fn refuses_what_it_cannot_read_back(backend: Backend, label: &str) {
    use amethystate::StoreExt;

    let path = TempPath::new("nonfinite_refused");
    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();

    let state = Readings::new_with(&store);
    state.ratio().set(5.0);

    let refused = state
        .ratio()
        .durable()
        .set(f64::NAN)
        .expect_err(&format!("{label} took a value it cannot read back"));
    assert!(
        format!("{refused:?}").contains("NaN or an infinity"),
        "{label} said {refused:?}"
    );

    assert_eq!(state.ratio().get(), 5.0, "{label}");
    assert_eq!(state.ratio().try_get().unwrap(), 5.0, "{label}");
    assert_eq!(
        StoreExt::get::<f64>(&store, ratio_path()).unwrap(),
        Some(5.0),
        "{label}: the refused write reached the store"
    );
}

#[cfg(feature = "json")]
#[test]
fn json_refuses_it() {
    refuses_what_it_cannot_read_back(Backend::Json, "json");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_refuses_it_too_because_it_stores_json() {
    refuses_what_it_cannot_read_back(Backend::Sqlite, "sqlite");
}

#[cfg(all(feature = "redb", feature = "json"))]
#[test]
fn a_promise_to_stay_readable_on_json_refuses_it_where_it_would_fit() {
    let path = TempPath::new("nonfinite_promised");
    let store = StoreBuilder::new(path.path())
        .backend(Backend::Redb)
        .limits(|l| l.portable_across([Backend::Json]))
        .build()
        .unwrap();

    let state = Readings::new_with(&store);

    let refused = state
        .ratio()
        .durable()
        .set(f64::INFINITY)
        .expect_err("msgpack holds it, but the store promised json too");

    let WriteValue::WillNotEncode { why, .. } = &refused else {
        panic!("{refused}")
    };
    assert!(format!("{why:?}").contains("NaN or an infinity"));
}

#[cfg(all(feature = "redb", feature = "json"))]
#[test]
fn a_store_promising_only_engines_that_hold_it_takes_it() {
    let path = TempPath::new("nonfinite_promised_ok");
    let store = StoreBuilder::new(path.path())
        .backend(Backend::Redb)
        .limits(|l| l.portable_across([Backend::Redb]))
        .build()
        .unwrap();

    let state = Readings::new_with(&store);
    state.ratio().set(f64::NEG_INFINITY);

    assert!(state.ratio().get().is_infinite());
}

#[test]
fn what_each_engine_does_with_a_nan() {
    for backend in common::enabled_backends() {
        let path = TempPath::new("nonfinite_run");
        let store = StoreBuilder::new(path.path())
            .backend(backend)
            .build()
            .unwrap();

        //@act
        let readings = Readings::new_with(&store);
        readings.ratio().set(1.5);

        //@show the write
        let written = readings.ratio().durable().set(f64::NAN);
        //@show-end

        let after = readings.ratio().get();
        //@end

        let held = if after.is_nan() {
            "NaN".to_string()
        } else {
            after.to_string()
        };
        let outcome = match &written {
            Ok(()) => format!("taken, and the field holds {held}"),
            Err(refused) => refused.to_string(),
        };

        common::measured(&[
            ("engine", common::engine_name(backend)),
            ("the write", &outcome),
            ("what the field holds afterwards", &held),
        ]);
    }
}

#[cfg(feature = "json")]
#[test]
fn the_refusal_names_the_kind_it_is() {
    let path = TempPath::new("nonfinite_kind");
    let store = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .build()
        .unwrap();

    let refused = store
        .set(["loose", "ratio"], &f64::NAN)
        .expect_err("a raw write is refused the same way");

    assert!(
        matches!(refused, WriteValue::WillNotEncode { .. }),
        "{refused}"
    );
}
