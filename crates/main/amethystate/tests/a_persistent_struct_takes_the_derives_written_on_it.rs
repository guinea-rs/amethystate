use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;
use serde::{Deserialize, Serialize};

#[amethystate(prefix = "derived", mode = "persistent")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Derived {
    #[amestate(default = 7u16)]
    pub port: u16,
}

#[backends(all)]
fn a_persistent_struct_takes_the_derives_written_on_it(backend: Backend) {
    let at = TempPath::new("persistent_derived");
    let store = StoreBuilder::new(&at).backend(backend).build().unwrap();

    let loaded = Derived::load_with(&store).unwrap();
    let copied = loaded.clone();

    assert_eq!(copied.port, 7);
    assert!(format!("{copied:?}").starts_with("Derived"));
}
