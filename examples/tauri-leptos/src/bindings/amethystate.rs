// GENERATED AUTOMATICALLY. DO NOT EDIT.
use amethystate_arena::amethystate_framework_arena;

#[amethystate_framework_arena]
#[::amethystate::amethystate(prefix = "todos", target = "tauri-wasm")]
pub struct Todos {
    #[amestate(default = ::amethystate::serde_json::from_str(r#"1"#).unwrap_or_default())]
    pub next_id: u64,
    #[amestate(default = ::amethystate::serde_json::from_str(r#"false"#).unwrap_or_default())]
    pub hide_done: bool,
    pub lists: ReactiveMap < String, TodoList >,
    pub items: ReactiveMap < String, Todo >,
}

