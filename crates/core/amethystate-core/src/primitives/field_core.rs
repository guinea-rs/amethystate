use crate::change::Change;
use crate::primitives::signal::{Signal, SignalSubscription, held};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub trait FieldValue: DeserializeOwned + Serialize + Clone + Send + Sync + 'static {}
impl<T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static> FieldValue for T {}

/// A rule a field was declared with: it puts the value right where it has to.
pub type DeclaredRule<T> = Arc<dyn Fn(&mut T) + Send + Sync + 'static>;

pub struct FieldCore<T> {
    pub signal: Signal<T>,
    pub rules: Arc<Mutex<Vec<DeclaredRule<T>>>>,
}

impl<T> Clone for FieldCore<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            rules: self.rules.clone(),
        }
    }
}

impl<T: Clone + 'static> FieldCore<T> {
    pub fn new_with_signal(initial: Signal<T>) -> Self {
        Self {
            signal: initial,
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn new(initial: T) -> Self {
        Self::new_with_signal(Signal::new(initial))
    }

    /// Adds a rule every write goes through, for as long as the field lives:
    /// there is no handle to take it off.
    pub fn rule<F>(&self, rule: F)
    where
        F: Fn(&mut T) + Send + Sync + 'static,
    {
        held(&self.rules).push(Arc::new(rule));
    }

    /// Puts `value` through the declared rules, in the order they were added.
    pub fn ruled(&self, value: &mut T) {
        let rules = held(&self.rules).clone();
        for rule in rules {
            rule(value);
        }
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    #[track_caller]
    pub fn subscribe<F>(&self, callback: F) -> SignalSubscription
    where
        F: for<'a> Fn(&'a T) + Send + Sync + 'static,
    {
        self.signal.subscribe(callback)
    }

    #[track_caller]
    pub fn subscribe_with_source<F>(&self, callback: F) -> SignalSubscription
    where
        F: for<'a> Fn(&'a T, Option<Uuid>) + Send + Sync + 'static,
    {
        self.signal.subscribe_with_source(callback)
    }

    /// The change a write of `value` makes, with the declared rules applied.
    pub fn change(&self, value: T, source: Option<Uuid>) -> Change<T> {
        let mut change = Change {
            source,
            old_value: self.get(),
            new_value: value,
        };
        self.ruled(&mut change.new_value);
        change
    }
}
