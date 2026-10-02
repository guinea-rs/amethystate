#![cfg(feature = "json")]

use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;

mod common;

#[amethystate(prefix = "old")]
pub struct Old {
    #[amestate(default = "dark".to_string())]
    pub theme: String,
}

fn retiring(at: &TempPath) -> amethystate::MigrationReport {
    let (_, report) = StoreBuilder::new(at.path())
        .backend(Backend::Json)
        .migrations(|m| {
            m.for_prefix("legacy").step(1, "retire the section", |ctx| {
                ctx.delete("theme")?;
                Ok(())
            });
        })
        .migrate()
        .unwrap();
    report
}

fn with_a_record_of_the_old_section(name: &str, version: Option<u32>) -> TempPath {
    let at = TempPath::new(name);
    {
        let store = StoreBuilder::new(at.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        Old::new_with(&store).unwrap();
        store
            .set(StorePath::from_segments(["legacy", "theme"]), &"light")
            .unwrap();
        store.save_now().unwrap();
    }

    let meta = common::bookkeeping_of(at.path());
    let mut written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    let record = written["schema.old"].clone();
    let fields = written.as_object_mut().unwrap();
    fields.insert("schema.legacy".into(), record);
    if let Some(version) = version {
        fields.insert(
            "meta.legacy".into(),
            serde_json::json!({ "versions": { "": version } }),
        );
    }
    std::fs::write(&meta, written.to_string()).unwrap();
    at
}

#[test]
fn a_prefix_retired_by_a_step_stops_reporting_drift_once_it_ran() {
    let at = with_a_record_of_the_old_section("retired_prefix_drift", None);

    let first = retiring(&at);
    assert!(!first.has_failures(), "{first:?}");

    let again = retiring(&at);
    assert!(!again.has_drift(), "{again:?}");
}

#[test]
fn a_prefix_retired_before_this_release_stops_reporting_drift() {
    let at = with_a_record_of_the_old_section("retired_prefix_drift_stuck", Some(1));

    let report = retiring(&at);

    assert!(!report.has_drift(), "{report:?}");
    assert!(!retiring(&at).has_drift());
}
