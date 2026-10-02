//! What this struct tells the rest of the process about itself: the entries a
//! running program can walk to find every schema that was declared.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use super::path_literal;
use crate::amethystate::model::{Field, OnDelete, Placement, Schema, Shape};
use crate::ts_mapping::map_type_to_ts;

/// The inventory entries for this schema: one the store reads to know what
/// was declared where, and one a Tauri front end reads to write types for it.
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
    let declared = match &prefix {
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
    };

    let for_tauri = tauri_entry(crate_name, schema, prefix.as_deref());

    quote! {
        #declared
        #for_tauri
    }
}

fn resets_tokens(rule: Option<OnDelete>) -> TokenStream2 {
    match rule {
        Some(OnDelete::UseDefault) => quote!(::core::option::Option::Some(true)),
        Some(OnDelete::Keep) => quote!(::core::option::Option::Some(false)),
        None => quote!(::core::option::Option::None),
    }
}

/// What a frontend may write to the field, and what it reads once the key is
/// gone: checked and built against the field's own type, here, where the type
/// is known.
fn value_tokens(crate_name: &TokenStream2, field: &Field) -> (TokenStream2, TokenStream2) {
    let refuses = quote! { |_: &str| false };
    let nothing = quote! { || ::core::option::Option::None };

    let Shape::Stored { default, stored_as } = &field.shape else {
        return (refuses, nothing);
    };

    let ty = &field.ty;
    let how = match stored_as {
        Some(how) => super::init::stored_as(crate_name, ty, how),
        None => quote! { #crate_name::store::StoredAs::default() },
    };
    let seed = super::seed_tokens(default);

    let accepts = quote! {
        |json: &str| <#ty as #crate_name::shape::Kind>::accepts(json, #how)
    };
    let default = quote! {
        || <#ty as #crate_name::shape::Kind>::written(#seed, #how)
    };
    (accepts, default)
}

fn tauri_entry(crate_name: &TokenStream2, schema: &Schema, prefix: Option<&str>) -> TokenStream2 {
    if !cfg!(feature = "tauri") {
        return quote!();
    }

    let name = &schema.name;
    let named = name.to_string();
    let at = match prefix {
        Some(written) => quote! { Some(#written) },
        None => quote! { None },
    };
    let version = schema.version;
    let id = match &schema.id {
        Some(written) => {
            let written = written.value.as_str();
            quote! { Some(#written) }
        }
        None => quote! { None },
    };
    let struct_resets = resets_tokens(schema.rules.on_delete.as_ref().map(|at| at.value));

    let each = schema.fields.iter().map(|field| {
        let fname_str = field.ident.to_string();
        let stored = match &field.shape {
            Shape::Node { flattened: true } => String::new(),
            _ => field.stored.value.clone(),
        };
        let (ts_type, full_ts_type) = map_type_to_ts(field.ty.clone());

        let ty = &field.ty;
        let rust_type_str = quote!(#ty).to_string();

        let kind = match &field.shape {
            Shape::Volatile { .. } => quote! { #crate_name::tauri::FieldKind::Volatile },
            Shape::Node { .. } => quote! {
                #crate_name::tauri::FieldKind::Nested {
                    entry: &<#ty as #crate_name::tauri::Exported>::EXPORT,
                }
            },
            Shape::Stored { .. } => match crate::amethystate::model::written_map(&field.ty) {
                Some((key, value)) => {
                    let k_ts = map_type_to_ts(key.clone()).1;
                    let v_ts = map_type_to_ts(value.clone()).1;
                    let k_rust = quote!(#key).to_string();
                    let v_rust = quote!(#value).to_string();
                    quote! {
                        #crate_name::tauri::FieldKind::ReactiveMap {
                            key_type: #k_ts,
                            value_type: #v_ts,
                            key_rust_type: #k_rust,
                            value_rust_type: #v_rust,
                            accepts_key: |name: &str| {
                                <#key as #crate_name::ReactiveMapKey>::read(name).is_some()
                            },
                        }
                    }
                }
                None => quote! { #crate_name::tauri::FieldKind::Plain },
            },
        };

        let (accepts, default) = value_tokens(crate_name, field);
        let resets = resets_tokens(field.rules.on_delete.as_ref().map(|at| at.value));

        quote! {
            #crate_name::tauri::FieldExportMeta {
                name: #fname_str,
                stored: #stored,
                ts_type: #ts_type,
                full_ts_type: #full_ts_type,
                rust_type: #rust_type_str,
                kind: #kind,
                accepts: #accepts,
                default: #default,
                resets: #resets,
            }
        }
    });

    let submitted = prefix.map(|_| {
        quote! {
            #crate_name::inventory::submit! {
                <#name as #crate_name::tauri::Exported>::EXPORT
            }
        }
    });

    quote! {
        impl #crate_name::tauri::Exported for #name {
            const EXPORT: #crate_name::tauri::SchemaExportEntry =
                #crate_name::tauri::SchemaExportEntry {
                    prefix: #at,
                    struct_name: #named,
                    module_path: ::core::module_path!(),
                    id: #id,
                    version: #version,
                    resets: #struct_resets,
                    fields: &[
                        #(#each),*
                    ],
                };
        }

        #submitted
    }
}
