use amethystate::amethystate;
#[cfg(feature = "json")]
use amethystate::observability::Reason;
use amethystate::store::RuleContext;
use amethystate::store::builder::StoreBuilder;
use amethystate_core::test_utils::TempPath;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
mod common;

pub struct InstalledThemes(pub Vec<&'static str>);

//@show a rule on a field, and the world it is judged against
fn a_size_that_renders(size: &mut u8, _cx: &RuleContext) {
    *size = (*size).clamp(6, 72);
}

fn a_theme_that_is_installed(theme: &mut String, cx: &RuleContext) {
    let installed = cx.require::<InstalledThemes>();

    if !installed.0.contains(&theme.as_str()) {
        *theme = "dark".to_string();
    }
}

#[amethystate(prefix = "ruled_ui")]
pub struct Ui {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,

    #[amestate(default = "dark".to_string(), rule = a_theme_that_is_installed)]
    pub theme: String,
}
//@show-end

#[amethystate(prefix = "ruled_loaded", mode = "persistent")]
pub struct LoadedUi {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,
}

fn themes() -> InstalledThemes {
    InstalledThemes(vec!["dark", "solarized"])
}

#[cfg(not(target_arch = "wasm32"))]
fn said(panicked: Box<dyn std::any::Any + Send>) -> String {
    panicked
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panicked.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

pub struct Monitors(pub usize);

#[test]
fn a_context_says_which_types_it_was_given() {
    let path = TempPath::new("context_debug");
    let store = StoreBuilder::new(path.path())
        .context(themes())
        .context(Monitors(2))
        .build()
        .unwrap();

    insta::assert_snapshot!(format!("{:?}", store.context()));
}

#[test]
fn a_rule_that_puts_the_value_right_is_the_value_the_field_holds() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_repair");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["ruled_ui", "font_size"], &3u8)?;

    let ui = Ui::new_with(&store);

    assert_eq!(ui.font_size().get(), 6);
    assert_eq!(ui.font_size().try_get().unwrap(), 6);
    assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(6));

    Ok(())
}

#[test]
fn a_rule_that_puts_the_value_right_is_what_a_loaded_struct_holds() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_repair_loaded");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["ruled_loaded", "font_size"], &200u8)?;

    let ui = LoadedUi::load_with(&store);

    assert_eq!(ui.font_size, 72);
    assert_eq!(store.get::<u8>(["ruled_loaded", "font_size"])?, Some(72));

    Ok(())
}

#[test]
fn a_value_the_rule_has_no_quarrel_with_is_read_as_it_is() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_accepted");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["ruled_ui", "font_size"], &20u8)?;

    let ui = Ui::new_with(&store);

    assert_eq!(ui.font_size().try_get()?, 20);

    Ok(())
}

#[test]
fn a_rule_judges_the_value_against_what_the_application_gave() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_context");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["ruled_ui", "theme"], &"solarized".to_string())?;

    let ui = Ui::new_with(&store);

    assert_eq!(ui.theme().try_get()?, "solarized");

    Ok(())
}

#[test]
fn a_theme_the_application_does_not_have_is_replaced_in_the_field_and_the_store()
-> anyhow::Result<()> {
    let path = TempPath::new("field_rule_unknown_theme");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["ruled_ui", "theme"], &"midnight".to_string())?;

    let ui = Ui::new_with(&store);

    assert_eq!(ui.theme().try_get()?, "dark");
    assert_eq!(
        store.get::<String>(["ruled_ui", "theme"])?,
        Some("dark".to_string())
    );

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_rule_whose_input_nobody_gave_panics_naming_it_and_what_was_given() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_no_context");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["ruled_ui", "theme"], &"solarized".to_string())?;

    let Err(panicked) = catch_unwind(AssertUnwindSafe(|| Ui::new_with(&store))) else {
        panic!("a rule that needs a value nobody gave opened anyway");
    };

    assert_eq!(
        said(panicked),
        "no value provided for field_rule::InstalledThemes; nothing was given. A declared rule, \
         and a struct that opens its own way, are handed their values through \
         StoreBuilder::context"
    );

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_rule_whose_input_nobody_gave_names_what_the_store_was_given_instead() -> anyhow::Result<()> {
    struct Elsewhere(#[allow(dead_code)] u8);

    let path = TempPath::new("field_rule_other_context");
    let store = StoreBuilder::new(path.path())
        .context(Elsewhere(1))
        .build()?;

    store.set(["ruled_ui", "theme"], &"solarized".to_string())?;

    let Err(panicked) = catch_unwind(AssertUnwindSafe(|| Ui::new_with(&store))) else {
        panic!("a rule that needs a value nobody gave opened anyway");
    };

    assert!(
        said(panicked).contains(
            "given: field_rule::a_rule_whose_input_nobody_gave_names_what_the_store_was_given_instead::Elsewhere."
        ),
        "the panic does not name what the store was given"
    );

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_from_outside_arrives_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_external_repair");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = Ui::new_with(&store);
    ui.font_size().set(42);
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "200"))?;

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().try_get()?, 72);
    store.save_now()?;
    let reopened = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .build()?;
    assert_eq!(reopened.get::<u8>(["ruled_ui", "font_size"])?, Some(72));

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_from_outside_the_rule_has_no_quarrel_with_arrives() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_external_good");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = Ui::new_with(&store);
    ui.font_size().set(42);
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "18"))?;

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().try_get()?, 18);

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_the_watcher_brings_in_is_written_back_as_the_rule_put_it_right() -> anyhow::Result<()> {
    use amethystate::StoreBackend;

    let path = TempPath::new("field_rule_watcher_repair");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = Ui::new_with(&store);
    ui.font_size().set(42);
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "200"))?;
    store.reread_from_disk();

    assert_eq!(ui.font_size().try_get()?, 72);
    assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(72));

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn a_value_a_second_store_committed_is_put_right_the_same_as_a_hand_edit() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_second_store");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = Ui::new_with(&store);
    ui.font_size().set(42);
    store.save_now()?;

    {
        let other = StoreBuilder::new(path.path())
            .backend(common::text_backend())
            .context(themes())
            .build()?;
        other.set(["ruled_ui", "font_size"], &3u8)?;
        other.save_now()?;
    }

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().try_get()?, 6);
    assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(6));

    Ok(())
}

#[amethystate(prefix = "ruled_session")]
pub struct Session {
    #[amestate(volatile, default = 14u8, rule = a_size_that_renders)]
    pub zoom: u8,
}

#[test]
fn a_write_lands_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_write_repaired");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;
    let ui = Ui::new_with(&store);

    //@show a write the rule puts right
    ui.font_size().set(200);

    assert_eq!(ui.font_size().get(), 72);
    assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(72));
    //@show-end

    Ok(())
}

#[test]
fn a_value_written_past_the_field_is_held_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_kv_repaired");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;
    let ui = Ui::new_with(&store);

    store.set(["ruled_ui", "font_size"], &200u8)?;

    assert_eq!(ui.font_size().try_get()?, 72);
    assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(72));

    Ok(())
}

#[test]
fn a_volatile_field_holds_what_its_rule_left() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_volatile");
    let store = StoreBuilder::new(path.path()).build()?;
    let session = Session::new_with(&store);

    session.zoom().set(3);

    assert_eq!(session.zoom().get(), 6);

    Ok(())
}

#[test]
fn a_loaded_struct_saves_what_its_rule_put_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_save_repaired");
    let store = StoreBuilder::new(path.path()).build()?;
    let mut ui = LoadedUi::load_with(&store);

    ui.font_size = 200;
    ui.save()?;

    assert_eq!(ui.font_size, 72);
    assert_eq!(store.get::<u8>(["ruled_loaded", "font_size"])?, Some(72));

    Ok(())
}

static VOLUMES_JUDGED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn a_volume_in_range(volume: &mut u8, _cx: &RuleContext) {
    VOLUMES_JUDGED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    *volume = (*volume).min(100);
}

#[amethystate(prefix = "ruled_volume")]
pub struct Volume {
    #[amestate(default = 50u8, rule = a_volume_in_range)]
    pub volume: u8,
}

#[test]
fn a_value_the_rule_put_right_is_judged_once() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_judged_once");
    let store = StoreBuilder::new(path.path()).build()?;
    let volume = Volume::new_with(&store);
    VOLUMES_JUDGED.store(0, std::sync::atomic::Ordering::SeqCst);

    store.set(["ruled_volume", "volume"], &200u8)?;

    assert_eq!(volume.volume().get(), 100);
    assert_eq!(VOLUMES_JUDGED.load(std::sync::atomic::Ordering::SeqCst), 1);

    Ok(())
}

fn finite_or_unbounded(limit: &mut f64, _cx: &RuleContext) {
    if *limit > 1e300 {
        *limit = f64::INFINITY;
    }
}

#[amethystate(prefix = "ruled_limit")]
pub struct Limit {
    #[amestate(default = 1.0f64, rule = finite_or_unbounded)]
    pub limit: f64,
}

#[cfg(feature = "json")]
#[test]
fn a_correction_the_store_will_not_take_is_a_disagreement() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_not_written_back");
    let store = StoreBuilder::new(path.path())
        .backend(amethystate::store::builder::Backend::Json)
        .build()?;
    let limit = Limit::new_with(&store);

    let _ = store.set(["ruled_limit", "limit"], &1e301f64);

    assert!(matches!(
        limit.limit().try_get().unwrap_err().reason,
        Reason::NotWrittenBack(_)
    ));
    assert_eq!(store.get::<f64>(["ruled_limit", "limit"])?, Some(1e301));

    Ok(())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "json", feature = "toml", feature = "ron")
))]
#[test]
fn a_value_written_past_the_field_reaches_the_file_as_the_rule_put_it_right() -> anyhow::Result<()>
{
    let path = TempPath::new("field_rule_kv_repaired_flushed");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .disk(|d| d.debounce(std::time::Duration::from_millis(20)))
        .build()?;
    let _ui = Ui::new_with(&store);
    std::thread::sleep(std::time::Duration::from_millis(200));

    store.set(["ruled_ui", "font_size"], &3u8)?;

    let on_disk = || -> anyhow::Result<Option<u8>> {
        let reader = StoreBuilder::new(path.path())
            .backend(common::text_backend())
            .build()?;
        Ok(reader.get::<u8>(["ruled_ui", "font_size"])?)
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while on_disk()? != Some(6) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(on_disk()?, Some(6));

    Ok(())
}

#[test]
fn a_store_that_agrees_with_every_rule_opens_quietly() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_ordinary");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    let ui = Ui::new_with(&store);

    assert_eq!(ui.font_size().try_get()?, 14);
    assert_eq!(ui.theme().try_get()?, "dark");

    Ok(())
}
