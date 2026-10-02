use crate::path::StorePath;
use std::fmt::Write;

/// The Tauri event a frontend hears changes at `path` on.
pub fn event_channel(path: &StorePath) -> String {
    let mut name = String::from("amethystate://");
    for (at, level) in path.segments().enumerate() {
        if at > 0 {
            name.push(':');
        }
        for byte in level.as_str().bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' => name.push(byte as char),
                other => {
                    let _ = write!(name, "_{other:02x}");
                }
            }
        }
    }
    name
}

#[derive(Debug, Clone, Copy)]
pub enum FieldKind {
    Plain,
    /// A struct of its own, whose entry says what it holds.
    Nested {
        entry: &'static SchemaExportEntry,
    },
    Volatile,
    ReactiveMap {
        key_type: &'static str,
        value_type: &'static str,
        key_rust_type: &'static str,
        value_rust_type: &'static str,
        /// Whether a level under the map names a key of its key type.
        accepts_key: fn(&str) -> bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FieldExportMeta {
    pub name: &'static str,
    /// Where the field is stored under its holder, its levels joined by dots:
    /// the name after `rename_all` or `path`, and empty for a flattened node.
    pub stored: &'static str,
    pub ts_type: &'static str,
    pub full_ts_type: &'static str,
    pub rust_type: &'static str,
    pub kind: FieldKind,
    /// Whether a JSON text is a value this field holds; for a map, a value of
    /// one of its entries.
    pub accepts: fn(&str) -> bool,
    /// The JSON its declared default is stored as; `None` for anything but a
    /// plain field.
    pub default: fn() -> Option<String>,
    /// Whether it takes its default again once its key is gone, where the
    /// field said; `None` leaves it to its struct.
    pub resets: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaExportEntry {
    pub prefix: Option<&'static str>,
    pub struct_name: &'static str,
    /// The module the struct is declared in, which tells apart two of one name.
    pub module_path: &'static str,
    /// The line the struct is a version of, where it named one.
    pub id: Option<&'static str>,
    pub version: u32,
    /// Whether its fields take their default again once a key is gone, where
    /// the struct said; `None` leaves it to the struct holding it, and at the
    /// top to the store.
    pub resets: Option<bool>,
    pub fields: &'static [FieldExportMeta],
}

impl SchemaExportEntry {
    /// Whether `other` was made from the same declaration.
    pub fn is(&self, other: &SchemaExportEntry) -> bool {
        self.module_path == other.module_path && self.struct_name == other.struct_name
    }
}

/// What a struct tells a Tauri frontend about itself.
///
/// Implemented by `#[amethystate]` for every struct when the `tauri` feature is
/// on; a struct holding another reaches it through this rather than by name.
pub trait Exported {
    const EXPORT: SchemaExportEntry;
}

inventory::collect!(SchemaExportEntry);

/// Every struct with a place of its own, at the newest version of its line.
///
/// A line kept beside its older versions for a migration is one struct here:
/// the older ones describe places the store no longer has. It is fixed for the
/// life of the process, so it is walked on the first ask and handed out as a
/// slice afterwards.
pub fn exports() -> &'static [&'static SchemaExportEntry] {
    static COMPILED: std::sync::OnceLock<Vec<&'static SchemaExportEntry>> =
        std::sync::OnceLock::new();

    COMPILED.get_or_init(|| {
        let mut newest: Vec<&'static SchemaExportEntry> = Vec::new();
        for entry in inventory::iter::<SchemaExportEntry> {
            let line = |other: &&mut &'static SchemaExportEntry| match entry.id {
                Some(id) => other.id == Some(id),
                None => other.id.is_none() && other.prefix == entry.prefix,
            };
            match newest.iter_mut().find(line) {
                Some(kept) if kept.version < entry.version => *kept = entry,
                Some(kept) if kept.version == entry.version && !kept.is(entry) => {
                    newest.push(entry)
                }
                Some(_) => {}
                None => newest.push(entry),
            }
        }
        newest
    })
}
