use crate::store::{OnUnreadable, OpenStruct, Writer};
use amethystate_core::path::StorePath;
use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A rule every value a declared field takes goes through: the one read from
/// the store, the one an edit from outside brings in, and the one this process
/// writes.
///
/// Written as a bare `fn` in `#[amestate(rule = ..)]`, so it captures
/// nothing; what it needs from the application arrives through
/// [`RuleContext`], which [`StoreBuilder::context`](crate::StoreBuilder::context)
/// fills.
///
/// It takes the value by `&mut` and leaves it acceptable: a value out of range
/// is clamped, one that cannot be repaired is replaced. There is no refusing -
/// whatever the rule leaves is what the field holds. A value written here is
/// written as the rule left it; a value read from the store - as a struct is
/// built or loaded, or as an edit arrives - is written back as the rule left
/// it, so the store holds what the field holds.
pub type Rule<TValue> = fn(&mut TValue, &RuleContext);

/// Values the application handed the store for its declared rules.
///
/// A rule runs whenever a value arrives or is written - while the struct is
/// being built, for every edit the file watcher brings in, and for every write -
/// so what it reads has to be usable from whichever thread is doing that. That
/// is the whole of the `Send + Sync` bound, and the whole of the difference
/// from [`StoreBuilder::provide`](crate::StoreBuilder::provide), which hands a
/// value to a migration step that runs once, inside `build`, on the thread that
/// called it.
///
/// Keyed by [`TypeId`], so one value of each type. Two of the same thing want
/// a type that says which is which, which is also what makes the call site
/// legible.
#[derive(Default)]
pub struct RuleContext {
    values: HashMap<TypeId, Held>,
}

struct Held {
    type_name: &'static str,
    value: Arc<dyn Any + Send + Sync>,
}

impl RuleContext {
    pub(crate) fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.values.insert(
            TypeId::of::<T>(),
            Held {
                type_name: type_name::<T>(),
                value: Arc::new(value),
            },
        );
    }

    /// Borrows what the application gave for `T`, or `None` if it gave none.
    ///
    /// The twin of [`RuleContext::require`] for a rule that can do without it.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.values
            .get(&TypeId::of::<T>())
            .and_then(|held| held.value.downcast_ref::<T>())
    }

    /// Borrows what the application gave for `T`.
    ///
    /// # Panics
    ///
    /// When the application gave nothing for `T`. That is how the store was
    /// built, and it fails the same way on every run; the message lists what
    /// was given.
    pub fn require<T: Any + Send + Sync>(&self) -> &T {
        self.get::<T>().unwrap_or_else(|| {
            panic!(
                "no value provided for {}; {}. A declared rule, and a struct that opens its \
                 own way, are handed their values through StoreBuilder::context",
                type_name::<T>(),
                self.on_offer()
            )
        })
    }

    fn on_offer(&self) -> String {
        let mut names: Vec<&'static str> =
            self.values.values().map(|held| held.type_name).collect();
        names.sort_unstable();

        if names.is_empty() {
            "nothing was given".to_string()
        } else {
            format!("given: {}", names.join(", "))
        }
    }
}

impl fmt::Debug for RuleContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuleContext")
            .field("types", &self.on_offer())
            .finish()
    }
}

/// Puts a value read from the store through its rule, and says whether the
/// rule changed it - which is when the store has to be told.
///
/// Compared as JSON values before and after, since a field's type promises no
/// `PartialEq`, and a JSON object compares by its entries whatever their order:
/// a `HashMap` the rule rebuilt with the same entries is unchanged. A value
/// that will not encode as JSON - a map keyed by a tuple - is taken as changed,
/// and an engine drops a write of the bytes it already holds.
pub(crate) fn judged<TValue: serde::Serialize>(
    rule: Rule<TValue>,
    value: &mut TValue,
    context: &RuleContext,
) -> bool {
    let before = serde_json::to_value(&*value).ok();
    rule(value, context);

    match (before, serde_json::to_value(&*value).ok()) {
        (Some(before), Some(after)) => before != after,
        _ => true,
    }
}

/// A declared leaf on the path that loads a plain struct: read the way the
/// declaration says it is stored, and answered for the way it says to answer.
///
/// One door for the three decisions a persistent field carries - how the stored
/// form is read, what an undecodable value does, and what its rule puts right.
/// Spelling them out per field at the call site is what let the last of them
/// apply while the first was ignored and the second reached nobody.
pub(crate) fn load_declared<TValue>(
    store: &crate::Store,
    at: &StorePath,
    stored_as: crate::store::traits::StoredAs<TValue>,
    rule: Option<Rule<TValue>>,
    policy: OnUnreadable,
    default: impl FnOnce() -> TValue,
) -> Result<TValue, OpenStruct>
where
    TValue: serde::de::DeserializeOwned + serde::Serialize + 'static,
{
    let mut held = match crate::store::read_stored(store, at, stored_as) {
        Ok(Some(held)) => held,
        Ok(None) => return Ok(default()),
        Err(why) => {
            return match policy {
                OnUnreadable::Refuse => Err(OpenStruct::WillNotRead {
                    at: at.clone(),
                    why: why.into(),
                }),
                OnUnreadable::UseDefault => {
                    tracing::error!(
                        target: "amethystate",
                        path = %at,
                        "what is stored will not read back as this field's type, so the field \
                         was loaded on its default: {why:?}"
                    );
                    Ok(default())
                }
            };
        }
    };

    if let Some(rule) = rule
        && judged(rule, &mut held, store.context())
    {
        crate::store::write_stored(store, at, &held, stored_as, Writer::judged(None))?;
    }

    Ok(held)
}

/// The same leaf on the way out, put through its rule where it stands, so what
/// the rule puts right is what the struct in hand holds afterwards.
pub(crate) fn judge_declared<TValue>(
    store: &crate::Store,
    value: &mut TValue,
    rule: Option<Rule<TValue>>,
) {
    if let Some(rule) = rule {
        rule(value, store.context());
    }
}

/// A judged leaf written the way the declaration says it is stored, so a save
/// leaves what [`load_declared`] reads.
pub(crate) fn save_declared<TValue>(
    store: &crate::Store,
    at: &StorePath,
    value: &TValue,
    stored_as: crate::store::traits::StoredAs<TValue>,
) -> Result<(), crate::store::WriteValue>
where
    TValue: serde::Serialize + 'static,
{
    crate::store::write_stored(store, at, value, stored_as, Writer::judged(None))
        .map_err(|why| crate::store::WriteValue::from_store(at, why))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn rebuilt(map: &mut HashMap<String, u32>, _cx: &RuleContext) {
        *map = map
            .iter()
            .map(|(name, width)| (name.clone(), *width))
            .collect();
    }

    fn clamped(map: &mut HashMap<(u8, u8), u32>, _cx: &RuleContext) {
        map.values_mut()
            .for_each(|width| *width = (*width).min(100));
    }

    #[test]
    fn a_map_the_rule_rebuilds_with_the_same_entries_is_unchanged() {
        let mut widths: HashMap<String, u32> = (0..16).map(|n| (format!("column{n}"), n)).collect();

        assert!(!judged(rebuilt, &mut widths, &RuleContext::default()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_value_never_given_panics_naming_its_type_and_what_was_given() {
        struct Limit;

        let mut cx = RuleContext::default();
        cx.insert(7u8);

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cx.require::<Limit>();
        }))
        .expect_err("require gave something for a type nobody provided");
        let said = panicked
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panicked.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();

        assert_eq!(
            said,
            format!(
                "no value provided for {}; given: u8. A declared rule, and a struct that opens \
                 its own way, are handed their values through StoreBuilder::context",
                type_name::<Limit>()
            )
        );
    }

    #[test]
    fn a_value_given_is_the_one_required() {
        let mut cx = RuleContext::default();
        cx.insert(7u8);

        assert_eq!(*cx.require::<u8>(), 7);
    }

    #[test]
    fn a_correction_to_a_value_json_cannot_hold_counts_as_a_change() {
        let mut widths = HashMap::from([((0, 1), 500u32)]);

        assert!(judged(clamped, &mut widths, &RuleContext::default()));
    }
}
