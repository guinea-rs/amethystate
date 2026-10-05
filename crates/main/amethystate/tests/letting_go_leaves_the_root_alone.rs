#![cfg(feature = "json")]

use amethystate::amethystate;
use amethystate::store::OnUndeclared;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;

#[amethystate(as_root)]
pub struct Settings {
    #[amestate(default = "dark".to_string())]
    pub theme: String,
}

fn settle() {
    std::thread::sleep(std::time::Duration::from_millis(120));
}

#[test]
fn a_struct_at_the_root_does_not_make_every_key_declared() {
    let path = TempPath::new("letting_go_root");
    {
        let store = StoreBuilder::new(path.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        let _settings = Settings::new_with(&store);
        store.kv().set("note", &"kept by hand".to_string()).unwrap();
        store.set(["elsewhere", "note"], &"kept by hand").unwrap();
        store.save_now().unwrap();
    }
    settle();

    let (store, report) = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()
        .unwrap();

    assert!(!report.has_failures());
    assert_eq!(
        (
            store.get::<String>(["note"]).unwrap(),
            store.get::<String>(["elsewhere", "note"]).unwrap(),
            store.get::<String>(["theme"]).unwrap(),
        ),
        (
            Some("kept by hand".to_string()),
            Some("kept by hand".to_string()),
            Some("dark".to_string()),
        )
    );
}
