use amethystate::store::StorePath;
use amethystate::tauri::{FieldExportMeta, FieldKind, SchemaExportEntry};
use std::collections::HashMap;

/// The places a frontend may reach: the fields and maps the exported structs
/// declare, and nothing around them.
///
/// Built once from [`amethystate::tauri::exports`], so a struct the frontend
/// was generated for is reachable and a key no struct declares is not. A
/// volatile field has no place and is never in it.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    fields: HashMap<StorePath, Field>,
    maps: HashMap<StorePath, &'static FieldExportMeta>,
}

/// A plain field in scope, with what its declarations say about a key that goes.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub meta: &'static FieldExportMeta,
    /// Whether it takes its default again once its key is gone, where the
    /// field or a struct around it said; `None` leaves it to the store.
    pub resets: Option<bool>,
}

impl Scope {
    /// Every place the structs this binary exports declare.
    pub fn exported() -> Self {
        let mut scope = Self::default();
        for entry in amethystate::tauri::exports() {
            if let Some(at) = entry
                .prefix
                .and_then(|written| placed(&StorePath::root(), written))
            {
                scope.take(entry, at, None);
            }
        }
        scope
    }

    fn take(&mut self, entry: &'static SchemaExportEntry, at: StorePath, inherited: Option<bool>) {
        let resets = entry.resets.or(inherited);
        for field in entry.fields {
            let Some(own) = placed(&at, field.stored) else {
                continue;
            };
            match field.kind {
                FieldKind::Plain => {
                    self.fields.insert(
                        own,
                        Field {
                            meta: field,
                            resets: field.resets.or(resets),
                        },
                    );
                }
                FieldKind::ReactiveMap { .. } => {
                    self.maps.insert(own, field);
                }
                FieldKind::Nested { entry: held } => self.take(held, own, field.resets.or(resets)),
                FieldKind::Volatile => {}
            }
        }
    }

    /// The plain field at `path`.
    pub fn field(&self, path: &StorePath) -> Option<&Field> {
        self.fields.get(path)
    }

    /// Whether `path` is a map.
    pub fn is_map(&self, path: &StorePath) -> bool {
        self.maps.contains_key(path)
    }

    /// The map `path` is an entry of.
    fn map_of(&self, path: &StorePath) -> Option<&'static FieldExportMeta> {
        self.maps.get(&path.parent()?).copied()
    }

    /// Whether `path` is a field or an entry of a map: a place holding one value.
    pub fn holds(&self, path: &StorePath) -> bool {
        self.field(path).is_some() || self.map_of(path).is_some()
    }

    /// Whether `path` is somewhere a frontend may take away whole: a value, or
    /// a map with all its entries.
    pub fn clears(&self, path: &StorePath) -> bool {
        self.holds(path) || self.is_map(path)
    }

    /// Whether `json` is a value the place at `path` holds.
    pub fn accepts(&self, path: &StorePath, json: &str) -> bool {
        if let Some(field) = self.field(path) {
            return (field.meta.accepts)(json);
        }

        let Some(map) = self.map_of(path) else {
            return false;
        };
        let named = path.name().is_some_and(|key| match map.kind {
            FieldKind::ReactiveMap { accepts_key, .. } => accepts_key(key.as_str()),
            _ => false,
        });
        named && (map.accepts)(json)
    }
}

fn placed(at: &StorePath, written: &str) -> Option<StorePath> {
    match written {
        "" | "." => Some(at.clone()),
        written => StorePath::parse_joined(written)
            .ok()
            .map(|own| at.join(&own)),
    }
}
