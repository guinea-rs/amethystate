#![doc = include_str!("../README.md")]

use std::any::Any;

use amethystate::reactive::field::FieldValue;
use amethystate::{Field, ReactiveCell};
use guinea_app::timers::Changing;

/// A reactive value as something a guinea timer's period can follow.
pub trait IntoChanging: Sized {
    fn changing(self) -> Changes<Self>;
}

/// What [`IntoChanging::changing`] hands to `Period::follows`.
pub struct Changes<S>(S);

impl<T: FieldValue> IntoChanging for Field<T> {
    fn changing(self) -> Changes<Self> {
        Changes(self)
    }
}

impl<T: Clone + Send + Sync + 'static> IntoChanging for ReactiveCell<T> {
    fn changing(self) -> Changes<Self> {
        Changes(self)
    }
}

impl<T: FieldValue> Changing for Changes<Field<T>> {
    type Value = T;

    fn now(&self) -> Option<T> {
        Some(self.0.get())
    }

    fn watch(&self, changed: Box<dyn Fn() + Send + Sync>) -> Box<dyn Any> {
        Box::new(self.0.subscribe(move |_| changed()))
    }
}

impl<T: Clone + Send + Sync + 'static> Changing for Changes<ReactiveCell<T>> {
    type Value = T;

    fn now(&self) -> Option<T> {
        self.0.get()
    }

    fn watch(&self, changed: Box<dyn Fn() + Send + Sync>) -> Box<dyn Any> {
        Box::new(self.0.subscribe(move |_| changed()))
    }
}
