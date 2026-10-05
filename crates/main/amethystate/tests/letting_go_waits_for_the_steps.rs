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
fn a_key_a_step_of_another_line_reads_is_there_when_it_runs() {
    let path = TempPath::new("letting_go_waits");
    {
        let store = StoreBuilder::new(path.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        let _source = Source::new_with(&store);
        store.set(["src", "moved"], &7u64).unwrap();
        store.save_now().unwrap();
    }
    settle();

    let (store, report) = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrations(|m| {
            m.for_prefix("dst").step(1, "take what src held", |ctx| {
                let moved = ctx.global_get::<u64>("src.moved")?.unwrap_or(999);
                ctx.set("moved", &moved)
            });
        })
        .migrate()
        .unwrap();

    assert!(!report.has_failures());
    assert_eq!(
        (
            store.get::<u64>(["dst", "moved"]).unwrap(),
            store.get::<u64>(["src", "moved"]).unwrap(),
        ),
        (Some(7), None)
    );
}
