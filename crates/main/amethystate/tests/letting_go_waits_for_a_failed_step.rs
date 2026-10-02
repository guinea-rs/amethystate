#![cfg(feature = "json")]

use amethystate::amethystate;
use amethystate::store::OnUndeclared;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;

#[amethystate(prefix = "src")]
pub struct Source {
    #[amestate(default = 1u64)]
    pub kept: u64,
}

fn settle() {
    std::thread::sleep(std::time::Duration::from_millis(120));
}

#[test]
fn an_open_where_a_step_failed_lets_go_of_nothing() {
    let path = TempPath::new("letting_go_failed");
    {
        let store = StoreBuilder::new(path.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        let _source = Source::new_with(&store).unwrap();
        store.set(["src", "moved"], &7u64).unwrap();
        store.save_now().unwrap();
    }
    settle();

    let opened = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrations(|m| {
            m.for_prefix("dst").step(1, "cannot take it yet", |ctx| {
                ctx.global_get::<String>("src.moved").map(|_| ())
            });
        })
        .migrate();
    assert!(opened.is_err());
    drop(opened);
    settle();

    let store = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .build()
        .unwrap();
    assert_eq!(store.get::<u64>(["src", "moved"]).unwrap(), Some(7));
}
