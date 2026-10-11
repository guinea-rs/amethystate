use amethystate::amethystate;
use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate_core::test_utils::TempPath;
use amethystate_test_macros::backends;

mod inner {
    use amethystate::amethystate;

    #[amethystate]
    pub struct Window {
        #[amestate(default = 800u32)]
        pub width: u32,
    }
}

use inner::Window;

#[amethystate(prefix = "ui")]
pub struct Ui {
    #[amestate(nested)]
    pub window: Window,
}

#[backends(all)]
fn a_nested_struct_named_through_an_import_round_trips(backend: Backend) {
    let path = TempPath::new("nested_through_use");

    {
        let store = StoreBuilder::new(path.path())
            .backend(backend)
            .build()
            .unwrap();
        let ui = Ui::new_with(&store);
        ui.window().width().set(1280);
        store.save_now().unwrap();
    }

    let store = StoreBuilder::new(path.path())
        .backend(backend)
        .build()
        .unwrap();
    let ui = Ui::new_with(&store);

    assert_eq!(ui.window().width().get(), 1280);
}
