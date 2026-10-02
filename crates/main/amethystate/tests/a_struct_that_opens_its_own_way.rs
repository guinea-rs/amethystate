use amethystate::store::builder::StoreBuilder;
use amethystate::store::{Invalid, OpenStruct};
use amethystate::{AmeStateSlice, Open, Schema, Store, amethystate};
use amethystate_core::test_utils::TempPath;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

pub struct Machine {
    pub cores: u32,
}

//@show opening any struct that has a place of its own
fn opened<S: Open>(store: &Store) -> Result<S, OpenStruct> {
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
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>()?;
        let settings = <Self as Schema>::open(store)?;

        if settings.workers().get() == 0 {
            settings.workers().set(machine.cores)?;
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
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>()?;
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
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        let window = <Self as Schema>::open(store)?;

        if window.min().get() > window.max().get() {
            return Err(Invalid::new("the smallest window is wider than the largest").into());
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

    let plain: Plain = opened(&store).unwrap();

    assert_eq!(plain.port().get(), 8080);
}

#[test]
fn a_struct_that_opens_its_own_way_is_opened_by_what_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand");

    let settings: AgentSettings = opened(&store).unwrap();

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn the_inherent_constructor_goes_through_the_one_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand_inherent");

    let settings = AgentSettings::new_with(&store).unwrap();

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn a_struct_loaded_as_a_slice_goes_through_the_one_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "opened_by_hand_slice");

    let settings = <AgentSettings as AmeStateSlice>::load_slice(&store).unwrap();

    assert_eq!(settings.workers().get(), 6);
}

#[test]
fn a_loaded_struct_that_opens_its_own_way_is_loaded_by_what_its_author_wrote() {
    let (_path, store) = a_machine_with(6, "loaded_by_hand");

    let through_the_trait: KeptAgent = opened(&store).unwrap();
    let inherent = KeptAgent::load_with(&store).unwrap();

    assert_eq!((through_the_trait.workers, inherent.workers), (6, 6));
}

#[test]
fn a_value_the_constructor_requires_and_was_not_given_declines_the_open() {
    let path = TempPath::new("opened_by_hand_without_a_machine");
    let store = StoreBuilder::new(path.path()).build().unwrap();

    let Err(refused) = AgentSettings::new_with(&store) else {
        panic!("a constructor that requires a machine opened without one");
    };

    assert!(matches!(refused, OpenStruct::Declined(_)));
    assert_eq!(
        refused.to_string(),
        "the struct's own constructor declined to open it: no value provided for \
         a_struct_that_opens_its_own_way::Machine; nothing was given. A declared rule, and a \
         struct that opens its own way, are handed their values through StoreBuilder::context"
    );
}

#[test]
fn a_constructor_that_turns_the_struct_down_says_why() {
    let path = TempPath::new("opened_by_hand_and_declined");
    let store = StoreBuilder::new(path.path()).build().unwrap();
    {
        let window = <Window as Schema>::open(&store).unwrap();
        window.min().set(2000).unwrap();
    }

    let Err(OpenStruct::Declined(said)) = Window::new_with(&store) else {
        panic!("a window wider at its smallest than at its largest opened");
    };

    assert_eq!(
        said.reason(),
        "the smallest window is wider than the largest"
    );
}
