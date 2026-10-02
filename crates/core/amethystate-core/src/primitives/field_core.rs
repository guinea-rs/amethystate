use crate::change::Change;
use crate::path::StorePath;
use crate::primitives::intercept::{InterceptDepth, InterceptDisposer, InterceptGuard};
use crate::primitives::signal::{Signal, SignalSubscription, held};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub trait FieldValue: DeserializeOwned + Serialize + Clone + Send + Sync + 'static {}
impl<T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static> FieldValue for T {}

/// A rule a field was declared with: it may put the value right, or turn it
/// down in its own words.
pub type DeclaredRule<T> = Arc<dyn Fn(&mut T) -> Result<(), String> + Send + Sync + 'static>;

pub struct FieldCore<T> {
    pub signal: Signal<T>,
    pub interceptors: Arc<
        Mutex<
            Vec<(
                u64,
                Arc<dyn Fn(Change<T>) -> Option<Change<T>> + Send + Sync + 'static>,
            )>,
        >,
    >,
    pub next_interceptor_id: Arc<AtomicUsize>,
    pub intercept_depth: InterceptDepth,
    pub rules: Arc<Mutex<Vec<DeclaredRule<T>>>>,
}

impl<T> Clone for FieldCore<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            interceptors: self.interceptors.clone(),
            next_interceptor_id: self.next_interceptor_id.clone(),
            intercept_depth: self.intercept_depth.clone(),
            rules: self.rules.clone(),
        }
    }
}

impl<T: Clone + 'static> FieldCore<T> {
    pub fn new_with_signal(initial: Signal<T>) -> Self {
        Self {
            signal: initial,
            interceptors: Arc::new(Mutex::new(Vec::new())),
            next_interceptor_id: Arc::new(AtomicUsize::new(0)),
            intercept_depth: InterceptDepth::default(),
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn new(initial: T) -> Self {
        Self {
            signal: Signal::new(initial),
            interceptors: Arc::new(Mutex::new(Vec::new())),
            next_interceptor_id: Arc::new(AtomicUsize::new(0)),
            intercept_depth: InterceptDepth::default(),
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Adds a rule every write has to pass, after the interceptors and for as
    /// long as the field lives: there is no handle to take it off.
    pub fn rule<F>(&self, rule: F)
    where
        F: Fn(&mut T) -> Result<(), String> + Send + Sync + 'static,
    {
        held(&self.rules).push(Arc::new(rule));
    }

    /// Puts `value` through the declared rules, in the order they were added.
    pub fn ruled(&self, value: &mut T) -> Result<(), String> {
        let rules = held(&self.rules).clone();
        for rule in rules {
            rule(value)?;
        }
        Ok(())
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

    pub fn intercept<F>(&self, path: StorePath, callback: F) -> InterceptDisposer
    where
        F: Fn(Change<T>) -> Option<Change<T>> + Send + Sync + 'static,
    {
        let id = self.next_interceptor_id.fetch_add(1, Ordering::Relaxed);
        held(&self.interceptors).push((id as u64, Arc::new(callback)));

        let interceptors = self.interceptors.clone();
        InterceptDisposer {
            id: id as u64,
            path: path.clone(),
            cleanup: Arc::new(move |id| {
                held(&interceptors).retain(|(i, _)| *i != id);
            }),
        }
    }

    pub fn run_interceptors(
        &self,
        path: StorePath,
        value: T,
        source: Option<Uuid>,
    ) -> Result<Change<T>, crate::primitives::intercept::Refusal> {
        use crate::primitives::intercept::Refusal;

        let mut change = Change {
            source,
            old_value: self.get(),
            new_value: value,
        };

        let Some(_guard) = InterceptGuard::enter(&self.intercept_depth, path) else {
            return Err(Refusal::Recursed);
        };

        let interceptors = held(&self.interceptors).clone();
        for (_, interceptor) in interceptors {
            if let Some(new_change) = interceptor(change.clone()) {
                change = new_change;
            } else {
                return Err(Refusal::Said(
                    "refused by an interceptor on the field".to_string(),
                ));
            }
        }

        self.ruled(&mut change.new_value).map_err(Refusal::Ruled)?;

        Ok(change)
    }
}
