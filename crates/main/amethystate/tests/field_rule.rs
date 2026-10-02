use amethystate::amethystate;
use amethystate::observability::Reason;
use amethystate::store::builder::StoreBuilder;
use amethystate::store::{Invalid, OpenStruct, RuleContext, WriteValue};
use amethystate_core::test_utils::TempPath;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
mod common;

pub struct InstalledThemes(pub Vec<&'static str>);

//@show a rule on a field, and the world it is judged against
fn a_size_that_renders(size: &mut u8, _cx: &RuleContext) -> Result<(), Invalid> {
    if *size >= 6 {
        Ok(())
    } else {
        Err(Invalid::new("a font size below 6 renders nothing"))
    }
}

fn a_theme_that_is_installed(theme: &mut String, cx: &RuleContext) -> Result<(), Invalid> {
    let installed = cx.require::<InstalledThemes>()?;

    if installed.0.contains(&theme.as_str()) {
        Ok(())
    } else {
        Err(Invalid::new(format!(
            "no theme called {theme} is installed"
        )))
    }
}

#[amethystate(prefix = "checked_lenient", on_unreadable = UseDefault)]
pub struct LenientUi {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,

    #[amestate(default = "dark".to_string(), rule = a_theme_that_is_installed)]
    pub theme: String,
}
//@show-end

#[amethystate(prefix = "checked_strict")]
pub struct StrictUi {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,
}

#[amethystate(prefix = "checked_loaded", mode = "persistent", on_unreadable = UseDefault)]
pub struct LoadedUi {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,
}

#[amethystate(prefix = "checked_loaded_strict", mode = "persistent")]
pub struct StrictLoadedUi {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,
}

fn themes() -> InstalledThemes {
    InstalledThemes(vec!["dark", "solarized"])
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

fn a_size_brought_into_range(size: &mut u8, _cx: &RuleContext) -> Result<(), Invalid> {
    *size = (*size).clamp(6, 72);
    Ok(())
}

#[amethystate(prefix = "checked_repaired")]
pub struct RepairedUi {
    #[amestate(default = 14u8, rule = a_size_brought_into_range)]
    pub font_size: u8,
}

#[amethystate(prefix = "checked_repaired_loaded", mode = "persistent")]
pub struct RepairedLoadedUi {
    #[amestate(default = 14u8, rule = a_size_brought_into_range)]
    pub font_size: u8,
}

#[test]
fn a_rule_that_puts_the_value_right_is_the_value_the_field_holds() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_repair");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_repaired", "font_size"], &3u8)?;

    let ui = RepairedUi::new_with(&store)?;

    assert_eq!(ui.font_size().get(), 6);
    assert_eq!(ui.font_size().try_get().unwrap(), 6);
    assert_eq!(store.get::<u8>(["checked_repaired", "font_size"])?, Some(6));

    Ok(())
}

#[test]
fn a_rule_that_puts_the_value_right_is_what_a_loaded_struct_holds() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_repair_loaded");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_repaired_loaded", "font_size"], &200u8)?;

    let ui = RepairedLoadedUi::load_with(&store)?;

    assert_eq!(ui.font_size, 72);
    assert_eq!(
        store.get::<u8>(["checked_repaired_loaded", "font_size"])?,
        Some(72)
    );

    Ok(())
}

#[test]
fn a_value_the_rule_refuses_does_not_open_a_strict_struct() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_strict");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_strict", "font_size"], &3u8)?;

    match StrictUi::new_with(&store).unwrap_err() {
        OpenStruct::Refused { said, .. } => {
            assert_eq!(&*said, "a font size below 6 renders nothing")
        }
        other => panic!("{other}"),
    }

    Ok(())
}

#[test]
fn a_refused_open_hands_over_the_path_and_the_reason() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_matched");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_strict", "font_size"], &3u8)?;

    //@show telling one refused open from another
    match StrictUi::new_with(&store) {
        Ok(_) => {}
        Err(OpenStruct::Refused { at, said }) => eprintln!("{at} will not do: {said}"),
        Err(OpenStruct::WillNotRead { at, why }) => eprintln!("{at} is unreadable: {why}"),
        Err(OpenStruct::Taken(taken)) => {
            eprintln!("{} already holds {}", taken.held_by, taken.at)
        }
        Err(other) => return Err(other.into()),
    }
    //@show-end

    Ok(())
}

#[test]
fn try_get_hands_over_the_same_facts_the_open_would_have() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_try_get_matched");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "font_size"], &3u8)?;

    let ui = LenientUi::new_with(&store)?;

    //@show asking a field what the store disagrees with
    let held = match ui.font_size().try_get() {
        Ok(size) => size,
        Err(no) => {
            match no.reason {
                Reason::Refused(said) => eprintln!("running on the default: {said}"),
                Reason::WillNotRead(said) => eprintln!("{} will not decode: {said}", no.at),
                Reason::Occupied(said) => eprintln!("{} was already taken: {said}", no.at),
                _ => eprintln!("{} is not what the store has", no.at),
            }

            ui.font_size().get()
        }
    };
    //@show-end

    assert_eq!(held, 14);

    Ok(())
}

#[test]
fn a_refused_value_takes_the_default_and_try_get_says_why() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_lenient");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "font_size"], &3u8)?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.font_size().get(), 14);
    assert!(ui.font_size().try_get().is_err());
    assert!(ui.theme().try_get().is_ok());

    Ok(())
}

#[test]
fn a_refused_value_is_left_where_it_is() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_untouched");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "font_size"], &3u8)?;
    let _ui = LenientUi::new_with(&store)?;

    assert_eq!(store.get::<u8>(["checked_lenient", "font_size"])?, Some(3));

    Ok(())
}

#[test]
fn a_value_the_rule_accepts_is_read_as_it_is() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_accepted");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "font_size"], &20u8)?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.font_size().try_get()?, 20);

    Ok(())
}

#[test]
fn a_rule_judges_the_value_against_what_the_application_gave() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_context");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "theme"], &"solarized".to_string())?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.theme().try_get()?, "solarized");

    Ok(())
}

#[test]
fn a_theme_the_application_does_not_have_is_refused() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_unknown_theme");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    store.set(["checked_lenient", "theme"], &"midnight".to_string())?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.theme().get(), "dark");
    assert!(ui.theme().try_get().is_err());

    Ok(())
}

#[test]
fn a_rule_whose_input_nobody_gave_refuses_the_value() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_no_context");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_lenient", "theme"], &"solarized".to_string())?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.theme().get(), "dark");

    let said = match ui.theme().try_get().unwrap_err().reason {
        Reason::Refused(said) => said,
        other => panic!("{other:?}"),
    };

    assert!(
        said.contains("InstalledThemes"),
        "the refusal does not name what the rule asked for: {said}"
    );
    assert!(
        said.contains("nothing was given"),
        "the refusal does not say what was on offer: {said}"
    );

    Ok(())
}

#[test]
fn a_refusal_says_what_the_store_was_given_instead() -> anyhow::Result<()> {
    struct Elsewhere(#[allow(dead_code)] u8);

    let path = TempPath::new("field_rule_other_context");
    let store = StoreBuilder::new(path.path())
        .context(Elsewhere(1))
        .build()?;

    store.set(["checked_lenient", "theme"], &"solarized".to_string())?;

    let ui = LenientUi::new_with(&store)?;

    let said = match ui.theme().try_get().unwrap_err().reason {
        Reason::Refused(said) => said,
        other => panic!("{other:?}"),
    };

    assert!(
        said.contains("Elsewhere"),
        "the refusal does not name what the store was given: {said}"
    );

    Ok(())
}

#[test]
fn an_invalid_prints_the_reason_it_carries() {
    let refused = Invalid::new("a font size below 6 renders nothing");

    assert_eq!(refused.to_string(), "a font size below 6 renders nothing");
}

#[test]
fn a_loaded_struct_takes_the_default_over_a_refused_value() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_loaded");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_loaded", "font_size"], &3u8)?;

    let ui = LoadedUi::load_with(&store)?;

    assert_eq!(ui.font_size, 14);

    Ok(())
}

#[test]
fn a_strict_loaded_struct_refuses_to_load_at_all() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_loaded_strict");
    let store = StoreBuilder::new(path.path()).build()?;

    store.set(["checked_loaded_strict", "font_size"], &3u8)?;

    match StrictLoadedUi::load_with(&store) {
        Ok(_) => panic!("a value the rule refuses must not load"),
        Err(OpenStruct::Refused { said, .. }) => {
            assert_eq!(&*said, "a font size below 6 renders nothing")
        }
        Err(other) => panic!("{other}"),
    }

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_from_outside_that_the_rule_refuses_is_not_taken() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_external");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = LenientUi::new_with(&store)?;
    ui.font_size().set(42)?;
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "3"))?;

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().get(), 42);
    assert!(ui.font_size().try_get().is_err());
    assert_eq!(store.get::<u8>(["checked_lenient", "font_size"])?, Some(3));

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_from_outside_the_rule_accepts_arrives() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_external_good");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = LenientUi::new_with(&store)?;
    ui.font_size().set(42)?;
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "18"))?;

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().try_get()?, 18);

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_from_outside_arrives_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_external_repair");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .build()?;

    let ui = RepairedUi::new_with(&store)?;
    ui.font_size().set(42)?;
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "200"))?;

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().try_get()?, 72);
    store.save_now()?;
    let reopened = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .build()?;
    assert_eq!(
        reopened.get::<u8>(["checked_repaired", "font_size"])?,
        Some(72)
    );

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn an_edit_the_watcher_brings_in_is_written_back_as_the_rule_put_it_right() -> anyhow::Result<()> {
    use amethystate::StoreBackend;

    let path = TempPath::new("field_rule_watcher_repair");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .build()?;

    let ui = RepairedUi::new_with(&store)?;
    ui.font_size().set(42)?;
    store.save_now()?;

    let on_disk = std::fs::read_to_string(path.path())?;
    std::fs::write(path.path(), on_disk.replace("42", "200"))?;
    store.reread_from_disk();

    assert_eq!(ui.font_size().try_get()?, 72);
    assert_eq!(
        store.get::<u8>(["checked_repaired", "font_size"])?,
        Some(72)
    );

    Ok(())
}

#[cfg(any(feature = "json", feature = "toml", feature = "ron"))]
#[test]
fn a_value_a_second_store_committed_is_judged_the_same_as_a_hand_edit() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_second_store");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .context(themes())
        .build()?;

    let ui = LenientUi::new_with(&store)?;
    ui.font_size().set(42)?;
    store.save_now()?;

    {
        let other = StoreBuilder::new(path.path())
            .backend(common::text_backend())
            .context(themes())
            .build()?;
        other.set(["checked_lenient", "font_size"], &3u8)?;
        other.save_now()?;
    }

    store.set(["elsewhere", "poke"], &1u8)?;

    assert_eq!(ui.font_size().get(), 42);
    assert!(ui.font_size().try_get().is_err());

    Ok(())
}

#[amethystate(prefix = "ruled_session")]
pub struct Session {
    #[amestate(volatile, default = 14u8, rule = a_size_that_renders)]
    pub zoom: u8,
}

#[test]
fn a_write_the_rule_refuses_does_not_land() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_write_refused");
    let store = StoreBuilder::new(path.path()).build()?;
    let ui = StrictUi::new_with(&store)?;

    //@show a write the rule turns down
    match ui.font_size().set(3) {
        Ok(()) => {}
        Err(WriteValue::Refused { at, said }) => eprintln!("{at} will not take it: {said}"),
        Err(other) => return Err(other.into()),
    }
    //@show-end

    let refused = ui.font_size().set(3).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "a declared rule turned down the write to checked_strict.font_size: a font size below 6 renders nothing"
    );
    assert_eq!(ui.font_size().get(), 14);
    assert_eq!(store.get::<u8>(["checked_strict", "font_size"])?, Some(14));

    Ok(())
}

#[test]
fn a_write_lands_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_write_repaired");
    let store = StoreBuilder::new(path.path()).build()?;
    let ui = RepairedUi::new_with(&store)?;

    ui.font_size().set(200)?;

    assert_eq!(ui.font_size().get(), 72);
    assert_eq!(
        store.get::<u8>(["checked_repaired", "font_size"])?,
        Some(72)
    );

    Ok(())
}

#[test]
fn a_value_written_past_the_field_is_held_as_the_rule_put_it_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_kv_repaired");
    let store = StoreBuilder::new(path.path()).build()?;
    let ui = RepairedUi::new_with(&store)?;

    store.set(["checked_repaired", "font_size"], &200u8)?;

    assert_eq!(ui.font_size().try_get()?, 72);
    assert_eq!(
        store.get::<u8>(["checked_repaired", "font_size"])?,
        Some(72)
    );

    Ok(())
}

#[test]
fn a_value_written_past_the_field_that_the_rule_refuses_is_not_taken() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_kv_refused");
    let store = StoreBuilder::new(path.path()).build()?;
    let ui = StrictUi::new_with(&store)?;

    let told = store.set(["checked_strict", "font_size"], &3u8);

    assert_eq!(
        told.unwrap_err().to_string(),
        "the change was stored, and a subscriber could not take it"
    );
    assert_eq!(ui.font_size().get(), 14);
    assert!(ui.font_size().try_get().is_err());
    assert_eq!(store.get::<u8>(["checked_strict", "font_size"])?, Some(3));

    Ok(())
}

#[test]
fn a_volatile_field_takes_only_what_its_rule_lets_through() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_volatile");
    let store = StoreBuilder::new(path.path()).build()?;
    let session = Session::new_with(&store)?;

    let refused = session.zoom().set(3);
    session.zoom().set(20)?;

    assert!(matches!(refused, Err(WriteValue::Refused { .. })));
    assert_eq!(session.zoom().get(), 20);

    Ok(())
}

#[test]
fn a_loaded_struct_will_not_save_what_its_rule_refuses() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_save_refused");
    let store = StoreBuilder::new(path.path()).build()?;
    let mut ui = StrictLoadedUi::load_with(&store)?;

    ui.font_size = 3;
    let refused = ui.save();

    assert!(matches!(refused, Err(WriteValue::Refused { .. })));
    assert_eq!(
        store.get::<u8>(["checked_loaded_strict", "font_size"])?,
        None
    );

    Ok(())
}

#[amethystate(prefix = "checked_loaded_pair", mode = "persistent")]
pub struct LoadedPair {
    #[amestate(default = 14u8, rule = a_size_brought_into_range)]
    pub accent: u8,

    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub body: u8,

    #[amestate(default = 14u8, rule = a_size_brought_into_range)]
    pub footer: u8,
}

#[test]
fn a_save_one_field_refuses_writes_none_of_the_others() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_save_half");
    let store = StoreBuilder::new(path.path()).build()?;
    let mut pair = LoadedPair::load_with(&store)?;

    pair.accent = 30;
    pair.body = 3;
    pair.footer = 30;
    let refused = pair.save();

    assert!(matches!(refused, Err(WriteValue::Refused { .. })));
    assert_eq!(
        (
            store.get::<u8>(["checked_loaded_pair", "accent"])?,
            store.get::<u8>(["checked_loaded_pair", "footer"])?,
        ),
        (None, None)
    );

    Ok(())
}

#[test]
fn a_loaded_struct_saves_what_its_rule_put_right() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_save_repaired");
    let store = StoreBuilder::new(path.path()).build()?;
    let mut ui = RepairedLoadedUi::load_with(&store)?;

    ui.font_size = 200;
    ui.save()?;

    assert_eq!(ui.font_size, 72);
    assert_eq!(
        store.get::<u8>(["checked_repaired_loaded", "font_size"])?,
        Some(72)
    );

    Ok(())
}

static VOLUMES_JUDGED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn a_volume_in_range(volume: &mut u8, _cx: &RuleContext) -> Result<(), Invalid> {
    VOLUMES_JUDGED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    *volume = (*volume).min(100);
    Ok(())
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
    let volume = Volume::new_with(&store)?;
    VOLUMES_JUDGED.store(0, std::sync::atomic::Ordering::SeqCst);

    store.set(["ruled_volume", "volume"], &200u8)?;

    assert_eq!(volume.volume().get(), 100);
    assert_eq!(VOLUMES_JUDGED.load(std::sync::atomic::Ordering::SeqCst), 1);

    Ok(())
}

fn finite_or_unbounded(limit: &mut f64, _cx: &RuleContext) -> Result<(), Invalid> {
    if *limit > 1e300 {
        *limit = f64::INFINITY;
    }

    Ok(())
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
    let limit = Limit::new_with(&store)?;

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
fn a_value_written_past_the_field_that_the_rule_refuses_still_reaches_the_file()
-> anyhow::Result<()> {
    let path = TempPath::new("field_rule_kv_refused_flushed");
    let store = StoreBuilder::new(path.path())
        .backend(common::text_backend())
        .disk(|d| d.debounce(std::time::Duration::from_millis(20)))
        .build()?;
    let _ui = StrictUi::new_with(&store)?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    let _ = store.set(["checked_strict", "font_size"], &3u8);

    let on_disk = || -> anyhow::Result<Option<u8>> {
        let reader = StoreBuilder::new(path.path())
            .backend(common::text_backend())
            .build()?;
        Ok(reader.get::<u8>(["checked_strict", "font_size"])?)
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while on_disk()? != Some(3) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(on_disk()?, Some(3));

    Ok(())
}

#[test]
fn a_store_that_agrees_with_every_rule_opens_quietly() -> anyhow::Result<()> {
    let path = TempPath::new("field_rule_ordinary");
    let store = StoreBuilder::new(path.path()).context(themes()).build()?;

    let ui = LenientUi::new_with(&store)?;

    assert_eq!(ui.font_size().try_get()?, 14);
    assert_eq!(ui.theme().try_get()?, "dark");

    Ok(())
}
