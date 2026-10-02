use amethystate::amethystate;
use amethystate_codegen::{CodegenRegistry, TauriVanillaCodegen};

#[amethystate]
pub struct TestNested {
    #[amestate(default = "nested_val".to_string())]
    pub name: String,
}

#[amethystate(prefix = "test_root")]
pub struct TestRoot {
    #[amestate(default = 42)]
    pub value: i32,

    #[amestate(default = "volatile_val".to_string(), volatile)]
    pub session: String,

    #[amestate(nested)]
    pub child: TestNested,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tab {
    pub title: String,
}

#[amethystate(prefix = "editor")]
pub struct Editor {
    #[amestate(default = Tab::default())]
    pub pinned: Tab,

    #[amestate(default = None)]
    pub last_closed: Option<Tab>,

    #[amestate(default = {})]
    pub open_tabs: amethystate::ReactiveMap<String, Tab>,
}

#[amethystate(prefix = "renamed", rename_all = "kebab-case")]
pub struct Renamed {
    #[amestate(default = false)]
    pub dark_mode: bool,

    #[amestate(default = 1, path = "window.width")]
    pub width: u32,

    #[amestate(nested, flatten)]
    pub inner: TestNested,
}

#[amethystate(prefix = "shapes")]
pub struct Shapes {
    pub gaps: Vec<Option<u8>>,
    pub counts: std::collections::HashMap<String, u32>,
    pub boxed: Box<String>,
    pub initial: char,
    pub pair: (u8, String),
    pub color: [u8; 4],
}

#[amethystate(prefix = "said \"so\"")]
pub struct Quoted {
    #[amestate(default = 0u8, path = "a \"b\"")]
    pub level: u8,
}

#[amethystate(as_root)]
pub struct Rooted {
    #[amestate(default = 0u8)]
    pub launches: u8,
}

mod v1 {
    use super::*;

    #[amethystate(prefix = "lined", version = 1)]
    pub struct Lined {
        #[amestate(default = String::new())]
        pub host: String,
    }
}

#[amethystate(prefix = "lined", version = 2)]
pub struct Lined {
    #[amestate(default = String::new())]
    pub address: String,
}

#[test]
fn a_field_is_exported_under_the_path_it_is_stored_at() {
    let renamed = amethystate::tauri::exports()
        .iter()
        .find(|entry| entry.struct_name == "Renamed")
        .expect("Renamed was not registered");

    let stored: Vec<(&str, &str)> = renamed
        .fields
        .iter()
        .map(|field| (field.name, field.stored))
        .collect();

    assert_eq!(
        stored,
        [
            ("dark_mode", "dark-mode"),
            ("width", "window.width"),
            ("inner", "")
        ]
    );
}

#[test]
fn test_rust_codegen_export() {
    let out_path = std::env::temp_dir().join("amethystate_test_export.rs");
    if out_path.exists() {
        let _ = std::fs::remove_file(&out_path);
    }

    let reg = CodegenRegistry::new().unwrap();
    reg.export_rust(&out_path, &TauriVanillaCodegen)
        .expect("Failed to export Rust bindings");

    assert!(out_path.exists());
    let rust_content =
        std::fs::read_to_string(&out_path).expect("Failed to read exported Rust file");

    insta::assert_snapshot!("rust_codegen_export", rust_content);
}

#[test]
fn every_struct_with_a_place_is_exported_once_at_its_newest_version() {
    let mut exported: Vec<(&str, Option<&str>, u32)> = amethystate::tauri::exports()
        .iter()
        .map(|entry| (entry.struct_name, entry.prefix, entry.version))
        .collect();
    exported.sort();

    assert_eq!(
        exported,
        [
            ("Editor", Some("editor"), 0),
            ("Lined", Some("lined"), 2),
            ("Quoted", Some("said \"so\""), 0),
            ("Renamed", Some("renamed"), 0),
            ("Rooted", Some("."), 0),
            ("Shapes", Some("shapes"), 0),
            ("TestRoot", Some("test_root"), 0),
        ]
    );
}

#[test]
fn a_held_struct_is_reached_through_its_holder() {
    use amethystate::tauri::{Exported, FieldKind};

    let root = <TestRoot as Exported>::EXPORT;
    let FieldKind::Nested { entry } = root.fields[2].kind else {
        panic!("`child` is not exported as a struct of its own");
    };

    assert!(entry.is(&<TestNested as Exported>::EXPORT));
    assert_eq!(entry.prefix, None);
    assert_eq!(
        entry
            .fields
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        ["name"]
    );
}

#[test]
fn test_typescript_codegen_export() {
    let out_path = std::env::temp_dir().join("amethystate_test_export.ts");
    if out_path.exists() {
        let _ = std::fs::remove_file(&out_path);
    }

    let reg = CodegenRegistry::new().unwrap();
    reg.export_ts(&out_path)
        .expect("Failed to export TS bindings");

    assert!(out_path.exists());
    let ts_content = std::fs::read_to_string(&out_path).expect("Failed to read exported TS file");

    insta::assert_snapshot!("typescript_codegen_export", ts_content);
}
