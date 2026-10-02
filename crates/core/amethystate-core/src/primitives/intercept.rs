use crate::path::StorePath;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) const MAX_INTERCEPT_DEPTH: usize = 10;

/// How deep the interceptors of one field or map are running inside each
/// other, counted on each thread by itself.
///
/// The limit is there for an interceptor that writes back into what it
/// guards, which is one thread calling itself. Writers on two threads at once
/// are not nested, and counting them together refuses the eleventh writer of a
/// busy map for a recursion nobody made.
#[derive(Clone, Default)]
pub struct InterceptDepth(Arc<()>);

thread_local! {
    static NESTED: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
}

impl InterceptDepth {
    /// Which field or map this is, for as long as a clone of it is alive -
    /// and a guard holds one.
    fn key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }

    fn deepen(&self, levels: usize) -> usize {
        NESTED.with(|nested| {
            let mut nested = nested.borrow_mut();
            let at = nested.entry(self.key()).or_insert(0);
            let was = *at;
            *at += levels;
            was
        })
    }

    fn rise(&self, levels: usize) {
        let _ = NESTED.try_with(|nested| {
            let mut nested = nested.borrow_mut();
            if let Some(at) = nested.get_mut(&self.key()) {
                *at = at.saturating_sub(levels);
                if *at == 0 {
                    nested.remove(&self.key());
                }
            }
        });
    }

    /// Stands this thread `levels` deep until the guard goes, as if that many
    /// interceptors were running inside each other.
    #[doc(hidden)]
    pub fn nested_on_this_thread(&self, levels: usize) -> InterceptGuard {
        self.deepen(levels);
        InterceptGuard {
            depth: self.clone(),
            levels,
        }
    }
}

impl std::fmt::Debug for InterceptDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InterceptDepth")
    }
}

/// Why a change did not get past the interceptors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// One of them turned it down, in these words.
    Said(String),

    /// They wrote back into what they guard deeper than a write may nest.
    Recursed,

    /// A rule the field was declared with turned it down, in these words.
    Ruled(String),
}

pub struct InterceptGuard {
    depth: InterceptDepth,
    levels: usize,
}

impl InterceptGuard {
    pub(crate) fn enter(depth: &InterceptDepth, path: StorePath) -> Option<Self> {
        let prev = depth.deepen(1);
        if prev >= MAX_INTERCEPT_DEPTH {
            depth.rise(1);
            tracing::warn!(
                target: "amethystate::intercept",
                path = %path,
                depth = prev + 1,
                "maximum intercept depth reached, skipping execution"
            );
            None
        } else {
            Some(Self {
                depth: depth.clone(),
                levels: 1,
            })
        }
    }
}

impl Drop for InterceptGuard {
    fn drop(&mut self) {
        self.depth.rise(self.levels);
    }
}

pub struct InterceptDisposer {
    pub id: u64,
    pub path: StorePath,
    pub(crate) cleanup: Arc<dyn Fn(u64) + Send + Sync + 'static>,
}

impl InterceptDisposer {
    pub fn remove(self) {
        (self.cleanup)(self.id);
    }
}

impl std::fmt::Debug for InterceptDisposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptDisposer")
            .field("id", &self.id)
            .field("path", &self.path)
            .finish()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::primitives::field_core::FieldCore;
    use std::time::Duration;

    #[test]
    fn writers_on_many_threads_at_once_are_not_a_recursion() {
        let core = FieldCore::new(0u32);
        let _slow = core.intercept(StorePath::segment("busy"), |change| {
            std::thread::sleep(Duration::from_millis(300));
            Some(change)
        });

        let writers: Vec<_> = (0..MAX_INTERCEPT_DEPTH as u32 + 2)
            .map(|n| {
                let core = core.clone();
                std::thread::spawn(move || {
                    core.run_interceptors(StorePath::segment("busy"), n, None)
                        .map(|_| ())
                })
            })
            .collect();

        let refused: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .filter(Result::is_err)
            .collect();
        assert_eq!(refused, Vec::new());
    }
}
