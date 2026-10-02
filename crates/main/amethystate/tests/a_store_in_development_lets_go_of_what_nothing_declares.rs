#![cfg(feature = "json")]

use amethystate::store::builder::{Backend, StoreBuilder};
use amethystate::store::{Fallbacks, OnUndeclared};
use amethystate::{MigrationReport, amethystate};
use amethystate_core::path::StorePath;
use amethystate_core::test_utils::TempPath;
use serde_json::{Value, json};

mod common;

#[amethystate]
pub struct Grouping {
    #[amestate(default = true)]
    pub by_user: bool,
}

#[amethystate(prefix = "processes")]
pub struct Processes {
    #[amestate(nested)]
    pub grouping: Grouping,
}

#[amethystate(prefix = "agents")]
pub struct Agents {
    #[amestate(default = 15u64)]
    pub connect_timeout: u64,
}

fn settle() {
    std::thread::sleep(std::time::Duration::from_millis(120));
}

fn todays_build(path: &TempPath, rules: impl FnOnce(Fallbacks) -> Fallbacks) -> MigrationReport {
    let (store, report) = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .rules(rules)
        .migrate()
        .unwrap();
    let processes = Processes::new_with(&store).unwrap();
    let agents = Agents::new_with(&store).unwrap();
    store.save_now().unwrap();
    drop((processes, agents, store));
    settle();
    report
}

fn field(name: &str, type_name: &str) -> Value {
    json!({
        "name": name,
        "type_name": type_name,
        "shape": { "role": "field", "optional": false },
    })
}

fn declared_at<'a>(held: &'a mut Value, prefix: &str) -> &'a mut Vec<Value> {
    let [only] = held[format!("schema.{prefix}")]
        .as_array_mut()
        .unwrap_or_else(|| panic!("nothing recorded at `{prefix}`"))
        .as_mut_slice()
    else {
        panic!("`{prefix}` records more than its one line");
    };
    only["fields"].as_array_mut().unwrap()
}

fn left_by_yesterdays_build(suffix: &str) -> TempPath {
    let path = TempPath::new(suffix);
    todays_build(&path, |rules| rules);

    let meta = common::bookkeeping_of(path.path());
    let mut held: Value = serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    declared_at(&mut held, "agents").extend([
        field("connect_timeout_secs", "u64"),
        field("scan_interval_ms", "u64"),
    ]);
    declared_at(&mut held, "processes")
        .iter_mut()
        .find(|declared| declared["name"] == "grouping")
        .expect("`processes` records no `grouping`")["shape"]["children"]
        .as_array_mut()
        .expect("`grouping` records no children")
        .push(field("expanded_groups", "Vec<String>"));
    std::fs::write(&meta, serde_json::to_string_pretty(&held).unwrap()).unwrap();

    {
        let store = StoreBuilder::new(path.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        store
            .set(["agents", "connect_timeout_secs"], &10u64)
            .unwrap();
        store.set(["agents", "scan_interval_ms"], &500u64).unwrap();
        store
            .set(["processes", "grouping", "expanded_groups"], &["ssh"])
            .unwrap();
        store
            .set(["agents", "scratch"], &"written by hand")
            .unwrap();
        store.set(["elsewhere", "note"], &"kept by hand").unwrap();
        store.save_now().unwrap();
    }
    settle();

    path
}

fn written_before_any_build_declared_it(suffix: &str) -> TempPath {
    let path = TempPath::new(suffix);
    todays_build(&path, |rules| rules);

    {
        let store = StoreBuilder::new(path.path())
            .backend(Backend::Json)
            .build()
            .unwrap();
        store.set(["agents", "retries"], &3u64).unwrap();
        store.save_now().unwrap();
    }
    settle();

    let meta = common::bookkeeping_of(path.path());
    let mut held: Value = serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    declared_at(&mut held, "agents").push(field("retries", "u64"));
    std::fs::write(&meta, serde_json::to_string_pretty(&held).unwrap()).unwrap();

    path
}

fn stored_keys(path: &TempPath) -> Vec<String> {
    let store = StoreBuilder::new(path.path())
        .backend(Backend::Json)
        .build()
        .unwrap();
    let mut keys: Vec<String> = store
        .scan_keys(StorePath::root())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    keys
}

#[test]
fn without_the_rule_every_start_reports_the_places_yesterday_left() {
    let path = left_by_yesterdays_build("undeclared_kept");

    assert!(todays_build(&path, |rules| rules).has_drift());
    assert!(todays_build(&path, |rules| rules).has_drift());
    assert_eq!(
        stored_keys(&path),
        [
            "agents.connect_timeout",
            "agents.connect_timeout_secs",
            "agents.scan_interval_ms",
            "agents.scratch",
            "elsewhere.note",
            "processes.grouping.by_user",
            "processes.grouping.expanded_groups",
        ]
    );
}

#[test]
fn a_build_that_renamed_and_dropped_fields_starts_without_a_migration() {
    let path = left_by_yesterdays_build("undeclared_dropped");

    let report = todays_build(&path, |rules| rules.on_undeclared(OnUndeclared::Drop));

    assert!(!report.has_drift());
    assert!(!report.has_failures());
    assert_eq!(
        stored_keys(&path),
        [
            "agents.connect_timeout",
            "elsewhere.note",
            "processes.grouping.by_user",
        ]
    );
}

#[test]
fn a_key_written_before_a_build_declared_it_goes_too() {
    let path = written_before_any_build_declared_it("undeclared_written_first");

    todays_build(&path, |rules| rules.on_undeclared(OnUndeclared::Drop));

    assert_eq!(
        stored_keys(&path),
        ["agents.connect_timeout", "processes.grouping.by_user"]
    );
}

#[test]
fn a_store_told_to_let_go_opens_with_nothing_to_report() -> Result<(), Box<dyn std::error::Error>> {
    let path = left_by_yesterdays_build("undeclared_shown");
    let path = path.path();

    //@show letting go of what nothing declares
    let (store, report) = StoreBuilder::new(path)
        //@hide
        .backend(Backend::Json)
        //@unhide
        .rules(|r| r.on_undeclared(OnUndeclared::Drop))
        .migrate()?;

    assert!(!report.has_drift());
    //@show-end

    drop(store);
    settle();
    Ok(())
}

#[test]
fn the_start_after_the_drop_is_quiet_too() {
    let path = left_by_yesterdays_build("undeclared_quiet_after");

    todays_build(&path, |rules| rules.on_undeclared(OnUndeclared::Drop));
    let next = todays_build(&path, |rules| rules.on_undeclared(OnUndeclared::Drop));

    assert!(!next.has_drift());
}
