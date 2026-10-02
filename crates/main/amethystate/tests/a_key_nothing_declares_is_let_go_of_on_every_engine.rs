use amethystate::store::OnUndeclared;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::{AmeData, amethystate, migrate};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;

#[amethystate(prefix = "agents")]
pub struct Agents {
    #[amestate(default = 15u64)]
    pub connect_timeout: u64,
}

mod v1 {
    use super::*;

    #[amethystate(prefix = "moving", version = 1)]
    pub struct Moving {
        #[amestate(default = 0u64)]
        pub before: u64,
    }
}

#[amethystate(prefix = "moving", version = 2)]
pub struct Moving {
    #[amestate(default = 0u64)]
    pub after: u64,
}

#[migrate]
#[rename(before => after)]
fn migrate_moving_v1_to_v2(
    old: AmeData<v1::Moving>,
) -> amethystate::MigrationResult<AmeData<Moving>> {
    Ok(AmeData::<Moving> { after: old.before })
}

fn stored_keys(store: &amethystate::Store) -> Vec<String> {
    let mut keys: Vec<String> = store
        .scan_keys(StorePath::root())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.retain(|key| !key.starts_with("moving"));
    keys.sort();
    keys
}

#[backends(files)]
fn a_key_under_a_declared_prefix_that_nothing_declares_goes(backend: Backend) {
    let path = TempPath::new("undeclared_every_engine");

    {
        let store = StoreBuilder::new(&path).backend(backend).build().unwrap();
        let _agents = Agents::new_with(&store).unwrap();
        store
            .set(["agents", "scratch"], &"written by hand")
            .unwrap();
        store.set(["elsewhere", "note"], &"kept by hand").unwrap();
        store.save_now().unwrap();
    }

    let (store, report) = StoreBuilder::new(&path)
        .backend(backend)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()
        .unwrap();

    assert!(!report.has_failures(), "{backend:?}: {report:?}");
    assert_eq!(
        stored_keys(&store),
        ["agents.connect_timeout", "elsewhere.note"],
        "{backend:?}"
    );
}

#[backends(files)]
fn a_prefix_with_a_step_due_keeps_what_the_step_reads(backend: Backend) {
    let path = TempPath::new("undeclared_step_due");

    {
        let store = StoreBuilder::new(&path).backend(backend).build().unwrap();
        let before = v1::Moving::new_with(&store).unwrap();
        before.before().set(7).unwrap();
        store.save_now().unwrap();
    }

    let (store, report) = StoreBuilder::new(&path)
        .backend(backend)
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()
        .unwrap();

    assert!(!report.has_failures(), "{backend:?}: {report:?}");
    assert_eq!(
        Moving::new_with(&store).unwrap().after().get(),
        7,
        "{backend:?}"
    );
}
