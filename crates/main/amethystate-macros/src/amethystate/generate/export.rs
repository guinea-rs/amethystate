//! What this struct tells the rest of the process about itself: the entries a
//! running program can walk to find every schema that was declared.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use super::path_literal;
use crate::amethystate::model::{Placement, Schema};

/// The inventory entry the store reads to know what was declared where.
pub(crate) fn entries(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    let named = schema.name.to_string();
    let prefix = schema.prefix.as_ref().map(Placement::path);

    let data_struct_name = format_ident!("{}_Data", schema.name);
    let version = schema.version;
    let id = match &schema.id {
        Some(written) => {
            let written = written.value.as_str();
            quote! { Some(#written) }
        }
        None => quote! { None },
    };

    // A struct with no prefix of its own is a component of one that has it:
    // its places are the holder's, reached through the field that holds it, and
    // everything reading this walks into a holder's fields already. An entry
    // for one would name no place and be skipped by every reader.
    match &prefix {
        None => quote! {},
        Some(written) => {
            let at = path_literal(crate_name, written);

            quote! {
                #crate_name::inventory::submit! {
                    #crate_name::schema::SchemaEntry {
                        prefix: #at,
                        id: #id,
                        struct_name: #named,
                        version: #version,
                        fields: <#data_struct_name as #crate_name::migration::fields::AmeStateFields>::FIELDS,
                    }
                }
            }
        }
    }
}
