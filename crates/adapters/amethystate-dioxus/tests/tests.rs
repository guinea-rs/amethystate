#![cfg(not(target_arch = "wasm32"))]

use amethystate::test_utils::unique_store;
use amethystate::{MapChange, Store, amethystate, uuid};
use amethystate_arena::{DefaultArena, FieldHandle, MapHandle};
use amethystate_dioxus::{
    AmeStateProvider, MapSignal, use_amethystate, use_field, use_map, use_map_entry,
    use_map_subscribe_any, use_map_subscribe_key,
};
use amethystate_macros_arena::amethystate_framework_arena;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Probe<T>(Arc<Mutex<Vec<T>>>);
impl<T> Probe<T> {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
    fn push(&self, val: T) {
        self.0.lock().unwrap().push(val);
    }
    fn last(&self) -> Option<T>
    where
        T: Clone,
    {
        self.0.lock().unwrap().last().cloned()
    }
    fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

impl<T> PartialEq for Probe<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Props)]
struct FieldTestProps {
    arena: DefaultArena,
    handle: FieldHandle<i32>,
    probe: Probe<i32>,
    setter_probe: Probe<Callback<i32>>,
}

impl PartialEq for FieldTestProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn FieldTestComponent(props: FieldTestProps) -> Element {
    let (val_signal, setter) = use_field(props.handle);
    props.probe.push(*val_signal.read());

    let setter1 = setter;
    let setter2 = setter;
    props.setter_probe.push(setter1);
    props.setter_probe.push(setter2);

    rsx! { div { "Field: {val_signal}" } }
}

#[tokio::test]
async fn test_use_field_requirements() {
    let (_at, store) = unique_store("field");
    let arena = DefaultArena::new();

    let field =
        amethystate::store::field_with_path(&store, ["field_1"], 10, uuid::Uuid::new_v4()).unwrap();
    let handle = arena.register_field(field);

    let probe = Probe::new();
    let setter_probe = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        |props: FieldTestProps| {
            use_context_provider(|| props.arena.clone());
            rsx! { FieldTestComponent { ..props } }
        },
        FieldTestProps {
            arena: arena.clone(),
            handle,
            probe: probe.clone(),
            setter_probe: setter_probe.clone(),
        },
    );

    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert_eq!(probe.last(), Some(10));

    let setter = setter_probe.last().unwrap();
    setter.call(42);

    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert_eq!(probe.last(), Some(42));
    assert_eq!(arena.get_field(handle), 42);

    let _ = arena.set_field(handle, 100);

    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert_eq!(probe.last(), Some(100));
}

#[derive(Clone, Props)]
struct MapTestProps {
    arena: DefaultArena,
    handle: MapHandle<String, String>,
    probe: Probe<HashMap<String, String>>,
    methods_probe: Probe<MapSignal<String, String>>,
}

impl PartialEq for MapTestProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn MapTestComponent(props: MapTestProps) -> Element {
    let map_signal = use_map(props.handle);
    props.probe.push(map_signal.entries.read().clone());
    props.methods_probe.push(map_signal);

    rsx! { div {} }
}

#[tokio::test]
async fn test_use_map_requirements() {
    let (_at, store) = unique_store("map");
    let arena = DefaultArena::new();

    let map = amethystate::store::reactive_map_with_path::<String, String>(
        &store,
        ["map_1"],
        HashMap::new(),
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    let handle = arena.register_map(map);

    let _ = arena.set_map_entry(handle, "key1".to_string(), "val1".to_string());

    let probe = Probe::new();
    let methods_probe = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        |props: MapTestProps| {
            use_context_provider(|| props.arena.clone());
            rsx! { MapTestComponent { ..props } }
        },
        MapTestProps {
            arena: arena.clone(),
            handle,
            probe: probe.clone(),
            methods_probe: methods_probe.clone(),
        },
    );

    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    let initial = probe.last().unwrap();
    assert_eq!(initial.get("key1").unwrap(), "val1");

    let methods = methods_probe.last().unwrap();

    methods.insert("key2".to_string(), "val2".to_string());
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert_eq!(probe.last().unwrap().get("key2").unwrap(), "val2");

    methods.remove("key1".to_string());
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert!(!probe.last().unwrap().contains_key("key1"));

    methods.clear();
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert!(probe.last().unwrap().is_empty());

    let _ = arena.set_map_entry(handle, "external".to_string(), "value".to_string());
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    assert_eq!(probe.last().unwrap().get("external").unwrap(), "value");
}

// #[amethystate_framework_arena]
// #[amethystate(prefix = "test")]
// pub struct MyTestState {
//     #[amestate(default = 0)]
//     pub id: usize,
// }

#[derive(Clone, Props)]
struct AmeStateProps {
    parent_probe: Probe<MyTestStateHandle>,
    child_probe: Probe<MyTestStateHandle>,
}

impl PartialEq for AmeStateProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn AmeStateParent(props: AmeStateProps) -> Element {
    let handle = use_amethystate::<MyTestState>();
    props.parent_probe.push(handle);

    rsx! {
        AmeStateChild {
            parent_probe: props.parent_probe.clone(),
            child_probe: props.child_probe.clone(),
        }
    }
}

#[component]
fn AmeStateChild(props: AmeStateProps) -> Element {
    let handle = use_amethystate::<MyTestState>();
    props.child_probe.push(handle);

    rsx! { div {} }
}

#[derive(Clone, Props)]
struct AmeStateTestWrapperProps {
    store: Store,
    parent_probe: Probe<MyTestStateHandle>,
    child_probe: Probe<MyTestStateHandle>,
}

impl PartialEq for AmeStateTestWrapperProps {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.store, &other.store)
            && self.parent_probe == other.parent_probe
            && self.child_probe == other.child_probe
    }
}

#[component]
fn amethystateTestWrapper(props: AmeStateTestWrapperProps) -> Element {
    rsx! {
        AmeStateProvider {
            store: props.store.clone(),
            AmeStateParent {
                parent_probe: props.parent_probe.clone(),
                child_probe: props.child_probe.clone(),
            }
        }
    }
}

#[tokio::test]
async fn test_use_amethystate_requirements() {
    let (_at, store) = unique_store("amethystate");

    let parent_probe = Probe::new();
    let child_probe = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        amethystateTestWrapper,
        AmeStateTestWrapperProps {
            store,
            parent_probe: parent_probe.clone(),
            child_probe: child_probe.clone(),
        },
    );

    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    let parent_handle = parent_probe.last().unwrap();
    let child_handle = child_probe.last().unwrap();

    assert!(parent_handle == child_handle);
}

#[derive(Clone, Props)]
struct SiblingProps {
    probe: Probe<MyTestStateHandle>,
}

impl PartialEq for SiblingProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn Sibling(props: SiblingProps) -> Element {
    props.probe.push(use_amethystate::<MyTestState>());
    rsx! { div {} }
}

#[derive(Clone, Props)]
struct SiblingsProps {
    store: Store,
    probe: Probe<MyTestStateHandle>,
}

impl PartialEq for SiblingsProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn Siblings(props: SiblingsProps) -> Element {
    rsx! {
        AmeStateProvider {
            store: props.store.clone(),
            Sibling { probe: props.probe.clone() }
            Sibling { probe: props.probe.clone() }
        }
    }
}

#[tokio::test]
async fn siblings_that_each_ask_for_a_slice_share_one() {
    let (_at, store) = unique_store("siblings");
    let probe = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        Siblings,
        SiblingsProps {
            store,
            probe: probe.clone(),
        },
    );
    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    let asked = probe.0.lock().unwrap().clone();
    assert_eq!(asked.len(), 2);
    assert!(asked[0] == asked[1]);
}

#[derive(Clone, Props)]
struct ChosenProps {
    field: FieldHandle<i32>,
    map: MapHandle<String, String>,
    entry: String,
    probe: Probe<(i32, Option<String>)>,
}

impl PartialEq for ChosenProps {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.map == other.map && self.entry == other.entry
    }
}

#[component]
fn Chosen(props: ChosenProps) -> Element {
    let (value, _) = use_field(props.field);
    let entry = use_map_entry(props.map, props.entry.clone());
    props.probe.push((*value.read(), entry.read().clone()));
    rsx! { div {} }
}

#[derive(Clone, Props)]
struct ChooserProps {
    arena: DefaultArena,
    fields: [FieldHandle<i32>; 2],
    map: MapHandle<String, String>,
    probe: Probe<(i32, Option<String>)>,
    chosen: Probe<Signal<usize>>,
}

impl PartialEq for ChooserProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn Chooser(props: ChooserProps) -> Element {
    use_context_provider(|| props.arena.clone());
    let chosen = use_signal(|| 0usize);
    use_hook(|| props.chosen.push(chosen));
    let at = *chosen.read();

    rsx! {
        Chosen {
            field: props.fields[at],
            map: props.map,
            entry: ["first", "second"][at].to_string(),
            probe: props.probe.clone(),
        }
    }
}

#[tokio::test]
async fn a_hook_given_another_handle_follows_it() {
    let (_at, store) = unique_store("chosen");
    let arena = DefaultArena::new();
    let field = |name: &str, value: i32| {
        let field =
            amethystate::store::field_with_path(&store, [name], value, uuid::Uuid::new_v4())
                .unwrap();
        arena.register_field(field)
    };
    let fields = [field("one", 1), field("two", 2)];
    let map = amethystate::store::reactive_map_with_path::<String, String>(
        &store,
        ["entries"],
        HashMap::from([
            ("first".to_string(), "a".to_string()),
            ("second".to_string(), "b".to_string()),
        ]),
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    let map = arena.register_map(map);
    let probe = Probe::new();
    let chosen = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        Chooser,
        ChooserProps {
            arena: arena.clone(),
            fields,
            map,
            probe: probe.clone(),
            chosen: chosen.clone(),
        },
    );
    vdom.rebuild(&mut dioxus::core::NoOpMutations);
    let settle = async |vdom: &mut VirtualDom| {
        tokio::task::yield_now().await;
        let _ =
            tokio::time::timeout(std::time::Duration::from_millis(100), vdom.wait_for_work()).await;
        vdom.render_immediate(&mut dioxus::core::NoOpMutations);
    };

    chosen.last().unwrap().set(1);
    settle(&mut vdom).await;
    let switched = probe.last();

    let _ = arena.set_field(fields[0], 10);
    let _ = arena.set_map_entry(map, "first".to_string(), "x".to_string());
    settle(&mut vdom).await;
    let after_the_old_ones_moved = probe.last();

    let _ = arena.set_field(fields[1], 20);
    let _ = arena.set_map_entry(map, "second".to_string(), "y".to_string());
    settle(&mut vdom).await;

    assert_eq!(
        [switched, after_the_old_ones_moved, probe.last()],
        [
            Some((2, Some("b".to_string()))),
            Some((2, Some("b".to_string()))),
            Some((20, Some("y".to_string()))),
        ]
    );
}

#[derive(Clone, Props)]
struct MapSubProps {
    arena: DefaultArena,
    handle: MapHandle<String, String>,
    any_changes: Arc<Mutex<Vec<MapChange<String, String>>>>,
    key_changes: Arc<Mutex<Vec<MapChange<String, String>>>>,
}

impl PartialEq for MapSubProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn MapSubComponent(props: MapSubProps) -> Element {
    let any_changes = props.any_changes.clone();
    use_map_subscribe_any(props.handle, move |change| {
        any_changes.lock().unwrap().push(change.clone());
    });

    let key_changes = props.key_changes.clone();
    use_map_subscribe_key(props.handle, "target".to_string(), move |change| {
        key_changes.lock().unwrap().push(change.clone());
    });

    rsx! { div {} }
}

#[tokio::test]
async fn test_map_sub_requirements() {
    let (_at, store) = unique_store("sub");
    let arena = DefaultArena::new();

    let map = amethystate::store::reactive_map_with_path::<String, String>(
        &store,
        ["map_2"],
        HashMap::new(),
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    let write_handle = arena.register_map(map);

    let any_changes = Arc::new(Mutex::new(Vec::new()));
    let key_changes = Arc::new(Mutex::new(Vec::new()));

    let mut vdom = VirtualDom::new_with_props(
        |props: MapSubProps| {
            use_context_provider(|| props.arena.clone());
            rsx! { MapSubComponent { ..props } }
        },
        MapSubProps {
            arena: arena.clone(),
            handle: write_handle,
            any_changes: any_changes.clone(),
            key_changes: key_changes.clone(),
        },
    );

    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    let _ = arena.set_map_entry(write_handle, "target".to_string(), "hello".to_string());
    tokio::task::yield_now().await;

    assert_eq!(any_changes.lock().unwrap().len(), 1);
    assert_eq!(key_changes.lock().unwrap().len(), 1);

    let _ = arena.set_map_entry(write_handle, "other".to_string(), "world".to_string());
    tokio::task::yield_now().await;

    assert_eq!(any_changes.lock().unwrap().len(), 2);
    assert_eq!(key_changes.lock().unwrap().len(), 1);

    drop(vdom);

    let _ = arena.set_map_entry(write_handle, "target".to_string(), "dropped".to_string());
    tokio::task::yield_now().await;

    assert_eq!(any_changes.lock().unwrap().len(), 2);
    assert_eq!(key_changes.lock().unwrap().len(), 1);
}

#[derive(Clone, Props)]
struct AllPrimitivesProps {
    arena: DefaultArena,
    field_handle: FieldHandle<i32>,
    map_handle: MapHandle<String, String>,

    field_probe: Probe<i32>,
    map_probe: Probe<HashMap<String, String>>,
    map_sub_any_probe: Probe<MapChange<String, String>>,
    map_sub_key_probe: Probe<MapChange<String, String>>,
}

impl PartialEq for AllPrimitivesProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn AllPrimitivesComponent(props: AllPrimitivesProps) -> Element {
    let (field_val, _field_setter) = use_field(props.field_handle);
    props.field_probe.push(*field_val.read());

    let map_signal = use_map(props.map_handle);
    props.map_probe.push(map_signal.entries.read().clone());

    let map_sub_any_probe = props.map_sub_any_probe.clone();
    use_map_subscribe_any(props.map_handle, move |change| {
        map_sub_any_probe.push(change.clone());
    });

    let map_sub_key_probe = props.map_sub_key_probe.clone();
    use_map_subscribe_key(props.map_handle, "target".to_string(), move |change| {
        map_sub_key_probe.push(change.clone());
    });

    rsx! { div {} }
}

#[derive(Clone, Props)]
struct AllPrimitivesToggleProps {
    arena: DefaultArena,
    field_handle: FieldHandle<i32>,
    map_handle: MapHandle<String, String>,

    field_probe: Probe<i32>,
    map_probe: Probe<HashMap<String, String>>,
    map_sub_any_probe: Probe<MapChange<String, String>>,
    map_sub_key_probe: Probe<MapChange<String, String>>,

    signal_probe: Probe<Signal<bool>>,
}

impl PartialEq for AllPrimitivesToggleProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[component]
fn AllPrimitivesToggleComponent(props: AllPrimitivesToggleProps) -> Element {
    use_context_provider(|| props.arena.clone());
    let toggle = use_signal(|| false);

    use_hook(|| {
        props.signal_probe.push(toggle);
    });

    rsx! {
        if *toggle.read() {
            AllPrimitivesComponent {
                arena: props.arena.clone(),
                field_handle: props.field_handle,
                map_handle: props.map_handle,
                field_probe: props.field_probe.clone(),
                map_probe: props.map_probe.clone(),
                map_sub_any_probe: props.map_sub_any_probe.clone(),
                map_sub_key_probe: props.map_sub_key_probe.clone(),
            }
        }
    }
}

#[tokio::test]
async fn test_all_primitives_simultaneous_lifecycle() {
    let (_at, store) = unique_store("all_primitives");
    let arena = DefaultArena::new();

    let field =
        amethystate::store::field_with_path(&store, ["field_all"], 10, uuid::Uuid::new_v4())
            .unwrap();
    let field_handle = arena.register_field(field);

    let map = amethystate::store::reactive_map_with_path::<String, String>(
        &store,
        ["map_all"],
        HashMap::new(),
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    let map_handle = arena.register_map(map);

    let field_probe = Probe::new();
    let map_probe = Probe::new();
    let map_sub_any_probe = Probe::new();
    let map_sub_key_probe = Probe::new();
    let signal_probe = Probe::new();

    let mut vdom = VirtualDom::new_with_props(
        AllPrimitivesToggleComponent,
        AllPrimitivesToggleProps {
            arena: arena.clone(),
            field_handle,
            map_handle,
            field_probe: field_probe.clone(),
            map_probe: map_probe.clone(),
            map_sub_any_probe: map_sub_any_probe.clone(),
            map_sub_key_probe: map_sub_key_probe.clone(),
            signal_probe: signal_probe.clone(),
        },
    );

    vdom.rebuild(&mut dioxus::core::NoOpMutations);

    let mut toggle_signal = signal_probe.last().expect("Exposed toggle signal missing");

    toggle_signal.set(true);
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;

    vdom.render_immediate(&mut dioxus::core::NoOpMutations);

    assert_eq!(field_probe.last(), Some(10));
    assert!(map_probe.last().unwrap().is_empty());

    let _ = arena.set_field(field_handle, 100);
    let _ = arena.set_map_entry(map_handle, "target".to_string(), "hello".to_string());
    let _ = arena.set_map_entry(map_handle, "other".to_string(), "world".to_string());

    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);

    assert_eq!(field_probe.last(), Some(100));

    assert_eq!(map_sub_any_probe.count(), 2);
    assert_eq!(map_sub_key_probe.count(), 1);

    toggle_signal.set(false);
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);

    let _ = arena.set_map_entry(
        map_handle,
        "target".to_string(),
        "dropped_value".to_string(),
    );
    tokio::task::yield_now().await;

    assert_eq!(map_sub_any_probe.count(), 2);
    assert_eq!(map_sub_key_probe.count(), 1);

    toggle_signal.set(true);
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);

    let _ = arena.set_map_entry(map_handle, "target".to_string(), "new_value".to_string());
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);

    assert_eq!(map_sub_any_probe.count(), 3);
    assert_eq!(map_sub_key_probe.count(), 2);

    toggle_signal.set(false);
    tokio::task::yield_now().await;
    let _ = vdom.wait_for_work().await;
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);
}

#[amethystate_framework_arena]
#[amethystate(prefix = "test")]
pub struct MyTestState {
    #[amestate(default = 0)]
    pub id: usize,
}
