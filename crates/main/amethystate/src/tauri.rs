pub use amethystate_core::scheme::*;
pub use amethystate_tauri::*;

use crate::store::StoredAs;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Whether `json` reads whole as a `T`, the way the field stores one.
#[doc(hidden)]
pub fn accepts<T: DeserializeOwned>(json: &str, stored_as: StoredAs<T>) -> bool {
    let mut de = serde_json::Deserializer::from_str(json);
    let read = match stored_as.read {
        Some(read_as) => read_as(&mut <dyn erased_serde::Deserializer>::erase(&mut de)).is_ok(),
        None => T::deserialize(&mut de).is_ok(),
    };
    read && de.end().is_ok()
}

/// The JSON `value` is stored as.
#[doc(hidden)]
pub fn written<T: Serialize>(value: &T, stored_as: StoredAs<T>) -> Option<String> {
    let Some(write_as) = stored_as.write else {
        return serde_json::to_string(value).ok();
    };

    let mut json = None;
    write_as(value, &mut |erased| {
        json = serde_json::to_string(erased).ok();
        Ok(())
    })
    .ok()?;
    json
}
