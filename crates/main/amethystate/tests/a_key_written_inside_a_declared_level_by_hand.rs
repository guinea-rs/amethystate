use amethystate::amethystate;
use amethystate::store::StorePath;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;

#[amethystate(prefix = "agents")]
pub struct Agents {
    #[amestate(default = 1000u64)]
    pub ping_interval_ms: u64,
}

fn opened(dir: &TempPath) -> amethystate::Store {
    std::fs::create_dir_all(&**dir).unwrap();
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
    let (store, _) = StoreBuilder::new(&path)
        .backend(Backend::Json)
        .migrate()
        .unwrap();
    store
}

fn listed(store: &amethystate::Store) -> Vec<String> {
    let mut keys: Vec<String> = store
        .scan_keys(StorePath::root())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    keys
}

#[test]
fn a_key_the_scan_lists_reads_back() {
    let dir = TempPath::new("by_hand_reads_back");
    let store = opened(&dir);

    assert_eq!(
        listed(&store),
        ["agents.ping_interval_ms", "agents.scan_interval_ms"]
    );
    assert_eq!(
        store.get::<u64>(["agents", "scan_interval_ms"]).unwrap(),
        Some(500)
    );
}

#[test]
fn a_key_the_scan_lists_can_be_deleted() {
    let dir = TempPath::new("by_hand_deleted");
    let store = opened(&dir);

    store.delete(["agents", "scan_interval_ms"]).unwrap();

    assert_eq!(listed(&store), ["agents.ping_interval_ms"]);
}
