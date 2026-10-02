use amethystate::amethystate;
use amethystate::store::OnUndeclared;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;

#[amethystate(prefix = "agents")]
pub struct Agents {
    #[amestate(default = 1000u64)]
    pub ping_interval_ms: u64,
}

#[backends(files)]
fn a_build_with_no_step_still_lets_go_of_what_nothing_declares(backend: Backend) {
    let path = TempPath::new("undeclared_no_step");

    {
        let (store, _) = StoreBuilder::new(&path).backend(backend).migrate().unwrap();
        let _agents = Agents::new_with(&store).unwrap();
        store.set(["agents", "scan_interval_ms"], &500u64).unwrap();
        store.save_now().unwrap();
    }

    let (store, report) = StoreBuilder::new(&path)
        .backend(backend)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()
        .unwrap();

    assert!(!report.has_failures(), "{backend:?}: {report:?}");
    let mut keys: Vec<String> = store
        .scan_keys(StorePath::root())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    assert_eq!(keys, ["agents.ping_interval_ms"], "{backend:?}");
}

#[test]
fn a_key_written_into_the_file_by_hand_goes_in_a_build_with_no_step() {
    let dir = TempPath::new("undeclared_no_step_by_hand");
    std::fs::create_dir_all(&*dir).unwrap();
    let path = dir.join("settings");

    {
        let (store, _) = StoreBuilder::new(&path)
            .backend(Backend::Json)
            .migrate()
            .unwrap();
        store.close().unwrap();
    }

    std::fs::write(
        dir.join("settings.json"),
        r#"{"agents":{"scan_interval_ms":500,"ping_interval_ms":1000}}"#,
    )
    .unwrap();

    let (store, report) = StoreBuilder::new(&path)
        .backend(Backend::Json)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()
        .unwrap();

    assert!(!report.has_failures(), "{report:?}");
    let mut keys: Vec<String> = store
        .scan_keys(StorePath::root())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    assert_eq!(keys, ["agents.ping_interval_ms"]);
}
