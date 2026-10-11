use amethystate::store::OpenStruct;
use amethystate::store::builder::StoreBuilder;
use amethystate::{AmeStateSlice, Open, Schema, Store, amethystate};
use amethystate_core::test_utils::TempPath;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

pub struct Machine {
    pub cores: u32,
}

//@show opening any struct that has a place of its own
fn opened<S: Open>(store: &Store) -> S {
    S::new_with(store)
}
//@show-end

#[amethystate(prefix = "opened_plainly")]
pub struct Plain {
    #[amestate(default = 8080u16)]
    pub port: u16,
}

//@show a struct that opens its own way
#[amethystate(prefix = "agent", open = manual)]
pub struct AgentSettings {
    #[amestate(default = 0u32)]
    pub workers: u32,
}

impl Open for AgentSettings {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>();
        let settings = <Self as Schema>::open(store)?;

        if settings.workers().get() == 0 {
            settings.workers().set(machine.cores);
        }

        Ok(settings)
    }
}
//@show-end

#[amethystate(prefix = "kept_agent", mode = "persistent", open = manual)]
pub struct KeptAgent {
    #[amestate(default = 0u32)]
    pub workers: u32,
}

impl Open for KeptAgent {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>();
        let mut settings = <Self as Schema>::open(store)?;

        if settings.workers == 0 {
            settings.workers = machine.cores;
        }

        Ok(settings)
    }
}

//@show an invariant between fields, checked where the struct is opened
#[amethystate(prefix = "window", open = manual)]
pub struct Window {
    #[amestate(default = 400u32)]
    pub min: u32,

    #[amestate(default = 1600u32)]
    pub max: u32,
}

impl Open for Window {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let window = <Self as Schema>::open(store)?;

        if window.min().get() > window.max().get() {
            window.max().set(window.min().get());
        }

        Ok(window)
    }
}
//@show-end

fn a_machine_with(cores: u32, name: &str) -> (TempPath, Store) {
    let path = TempPath::new(name);
    let store = StoreBuilder::new(path.path())
        .context(Machine { cores })
        .build()
        .unwrap();
    (path, store)
}

#[test]
fn a_struct_is_opened_through_the_trait_like_any_other() {
    let path = TempPath::new("opened_plainly");
    let store = StoreBuilder::new(path.path()).build().unwrap();

    let plain: Plain = opened(&store);

    assert_eq!(plain.port().get(), 8080);
}

#[test]
fn a_struct_that_opens_its_own_way_is_opened_by_what_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand");

    let settings: AgentSettings = opened(&store);

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn the_inherent_constructor_goes_through_the_one_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand_inherent");

    let settings = AgentSettings::new_with(&store);

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn the_inherent_try_constructor_goes_through_the_one_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand_inherent_try");

    let settings = AgentSettings::try_new_with(&store).unwrap();

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn a_struct_loaded_as_a_slice_goes_through_the_one_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand_slice");

    let settings = <AgentSettings as AmeStateSlice>::load_slice(&store);

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn a_loaded_struct_that_opens_its_own_way_is_loaded_by_what_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "loaded_by_hand");

    let through_the_trait: KeptAgent = opened(&store);
    let inherent = KeptAgent::load_with(&store);
    let tried = KeptAgent::try_load_with(&store).unwrap();

    assert_eq!(
        (through_the_trait.workers, inherent.workers, tried.workers),
        (6, 6, 6)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_value_the_constructor_requires_and_was_not_given_panics_naming_it() {
    let path = TempPath::new("opened_by_hand_without_a_machine");
    let store = StoreBuilder::new(path.path()).build().unwrap();

    let Err(panicked) = catch_unwind(AssertUnwindSafe(|| AgentSettings::new_with(&store))) else {
        panic!("a constructor that requires a machine opened without one");
    };
    let said = panicked
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();

    assert_eq!(
        said,
        "no value provided for a_struct_that_opens_its_own_way::Machine; nothing was given. \
         A declared rule, and a struct that opens its own way, are handed their values through \
         StoreBuilder::context"
    );
}

#[test]
fn a_constructor_puts_an_invariant_between_fields_right() {
    let path = TempPath::new("opened_by_hand_and_put_right");
    let store = StoreBuilder::new(path.path()).build().unwrap();
    {
        let window = <Window as Schema>::open(&store).unwrap();
        window.min().set(2000);
    }

    let window = Window::new_with(&store);

    assert_eq!((window.min().get(), window.max().get()), (2000, 2000));
}
