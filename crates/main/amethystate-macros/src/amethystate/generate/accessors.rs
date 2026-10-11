use crate::amethystate::generate::path_parts;
use crate::amethystate::model::{Field, Mode, Schema, Shape};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// The stored type of one field, as it appears in the reactive struct.
pub(crate) fn field_type(crate_name: &TokenStream2, field: &Field) -> TokenStream2 {
    let ty = &field.ty;

    match &field.shape {
        Shape::Node { .. } => quote! { ::std::sync::Arc<#ty> },
        Shape::Stored { .. } => quote! { <#ty as #crate_name::shape::Kind>::Handle },
        Shape::Volatile { .. } => quote! { #crate_name::Field<#ty> },
    }
}

pub(crate) fn struct_fields<'a>(
    crate_name: &'a TokenStream2,
    fields: &'a [Field],
) -> impl Iterator<Item = TokenStream2> + 'a {
    fields.iter().map(move |field| {
        let fname = &field.ident;
        let fvis = &field.vis;
        let ty = field_type(crate_name, field);
        let carried = &field.forwarded;

        quote! { #(#carried)* #fvis #fname: #ty }
    })
}

/// A getter per field, handing back a clone of what the struct holds.
pub(crate) fn methods(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    let each = schema.fields.iter().map(|field| {
        let fname = &field.ident;
        let held = field_type(crate_name, field);
        let carried = &field.forwarded;

        quote! {
            #(#carried)*
            pub fn #fname(&self) -> #held {
                self.#fname.clone()
            }
        }
    });

    quote! { #(#each)* }
}

/// The types this struct's constructor always constructs in turn.
///
/// A `nested` field is built unconditionally, so those are the edges a cycle
/// can run along. Nothing else is: a map recursing through its value type
/// decodes those values rather than constructing them.
fn construction_edges(crate_name: &TokenStream2, fields: &[Field]) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter(|field| matches!(field.shape, Shape::Node { .. }))
        .map(|field| {
            let ty = &field.ty;
            quote_spanned! {ty.span()=>
                let _: () = <#ty as #crate_name::AmeStateNode>::CONSTRUCTION_TERMINATES;
            }
        })
        .collect()
}

pub(crate) fn node_impl(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    if schema.mode == Mode::Persistent {
        return quote! {};
    }

    let name = &schema.name;
    let edges = construction_edges(crate_name, &schema.fields);
    let terminates = quote! {
        const CONSTRUCTION_TERMINATES: () = { #(#edges)* };
    };
    let force = quote! {
        const _: () = <#name as #crate_name::AmeStateNode>::CONSTRUCTION_TERMINATES;
    };

    quote! {
        impl #crate_name::AmeStateNode for #name {
            #terminates
        }

        #force
    }
}

/// Where this struct sits, as a constant on the type.
///
/// A struct meant to be embedded has none: it sits wherever its holder puts
/// it, which is a value rather than a constant.
pub(crate) fn scope(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    let Some(placement) = &schema.prefix else {
        return quote! {};
    };

    let name = &schema.name;
    let written = placement.path();
    let (segments, joined) = path_parts(&written);
    let id = match &schema.id {
        Some(written) => {
            let written = written.value.as_str();
            quote! { Some(#written) }
        }
        None => quote! { None },
    };

    quote! {
        impl #crate_name::StateScope for #name {
            const PATH: #crate_name::store::StorePath =
                #crate_name::store::StorePath::from_static(&[#(#segments),*], #joined);
            const KEY: &'static str = #joined;
            const ID: ::core::option::Option<&'static str> = #id;
        }
    }
}

/// How a struct with a place of its own is loaded and watched by callers that
/// know it only as a slice of the store.
///
/// A struct without a place has no `load_slice`: there is nowhere to load it
/// from until something says where.
pub(crate) fn slice_impl(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    if !schema.is_root() {
        return quote! {};
    }

    let name = &schema.name;
    let mode = schema.mode;

    let subs = if mode == Mode::Reactive {
        quote! {
            fn subscribe_all<F>(&self, callback: F) -> #crate_name::ReactiveScope
            where
                F: Fn() + Send + Sync + 'static,
            {
                self.subscribe_all(callback)
            }

            fn subscribe_all_external<F>(&self, callback: F) -> #crate_name::ReactiveScope
            where
                F: Fn() + Send + Sync + 'static,
            {
                self.subscribe_all_external(callback)
            }
        }
    } else {
        quote! {
            fn subscribe_all<F>(&self, _callback: F) -> #crate_name::ReactiveScope
            where
                F: Fn() + Send + Sync + 'static,
            {
                #crate_name::ReactiveScope::new()
            }

            fn subscribe_all_external<F>(&self, _callback: F) -> #crate_name::ReactiveScope
            where
                F: Fn() + Send + Sync + 'static,
            {
                #crate_name::ReactiveScope::new()
            }
        }
    };

    quote! {
        impl #crate_name::AmeStateSlice for #name {
            fn try_load_slice(store: &#crate_name::Store) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                <Self as #crate_name::store::Open>::try_new_with(store)
            }

            #subs
        }
    }
}

/// The struct opened as declared, and - unless its author writes one - the
/// `Open` that does nothing else.
///
/// A struct that loads plain data opens by loading it, so its mechanism is the
/// load; the rest open by building their fields.
pub(crate) fn opening(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    if !schema.is_root() {
        return quote! {};
    }

    let name = &schema.name;

    let declared = match schema.mode {
        Mode::Persistent => super::data::loaded(crate_name, schema),
        _ => quote! { Self::new_with_id(store, #crate_name::uuid::Uuid::new_v4()) },
    };

    let open = match schema.manual_open {
        Some(_) => quote! {},
        None => quote! {
            impl #crate_name::store::Open for #name {
                fn try_new_with(store: &#crate_name::Store) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                    <Self as #crate_name::store::Schema>::open(store)
                }
            }
        },
    };

    quote! {
        impl #crate_name::store::Schema for #name {
            fn open(store: &#crate_name::Store) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                #declared
            }
        }

        #open
    }
}

/// `new()` against the store this process installed globally.
///
/// A struct whose author writes its `Open` has none: reaching for the global
/// store is then a compile error rather than something a review has to catch.
pub(crate) fn global_new(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    if !schema.is_root() || schema.mode == Mode::Persistent || schema.manual_open.is_some() {
        return quote! {};
    }

    let name = &schema.name;

    quote! {
        impl #name {
            /// Opens the struct over the global store.
            ///
            /// # Panics
            ///
            /// Where `try_new` answers `Err`, with what it said.
            #[track_caller]
            pub fn new() -> Self {
                let store = #crate_name::global_store();
                Self::new_with(&store)
            }

            /// Opens the struct over the global store, or says why it would
            /// not open.
            pub fn try_new() -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                let store = #crate_name::global_store();
                Self::try_new_with(&store)
            }
        }
    }
}

pub(crate) fn constructor(crate_name: &TokenStream2, schema: &Schema) -> TokenStream2 {
    let init_fields = super::init::init_fields(crate_name, schema);

    if schema.is_root() {
        quote! {
            /// Opens the struct over `store` through its `Open`.
            ///
            /// # Panics
            ///
            /// Where `try_new_with` answers `Err`, with what it said.
            #[track_caller]
            pub fn new_with(store: &#crate_name::Store) -> Self {
                <Self as #crate_name::store::Open>::new_with(store)
            }

            /// Opens the struct over `store` through its `Open`, or says why it
            /// would not open.
            pub fn try_new_with(store: &#crate_name::Store) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                <Self as #crate_name::store::Open>::try_new_with(store)
            }

            /// Builds the struct as declared, under `instance_id` - the way
            /// `Schema::open` does, not through the struct's own `Open`.
            pub fn new_with_id(store: &#crate_name::Store, instance_id: #crate_name::uuid::Uuid) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                let __ame_fallbacks = store.fallbacks();
                Self::new_with_id_under(
                    store,
                    instance_id,
                    __ame_fallbacks.on_unreadable,
                    __ame_fallbacks.on_delete,
                    __ame_fallbacks.unreadable_entries,
                )
            }

            /// The same, told what the struct holding this one decided about a
            /// value it cannot read, a key removed under it, and an entry of a
            /// map that will not read.
            ///
            /// Whatever this struct declared for itself wins; these are what a
            /// field falls back to when neither it nor this struct said.
            pub fn new_with_id_under(
                store: &#crate_name::Store,
                instance_id: #crate_name::uuid::Uuid,
                __ame_on_unreadable: #crate_name::store::OnUnreadable,
                __ame_on_delete: #crate_name::store::OnDelete,
                __ame_unreadable_entries: #crate_name::store::UnreadableEntries,
            ) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                use #crate_name::{StoreBackend, StoreExt};
                let __amethystate_guard = #crate_name::store::instances::InstanceGuard::new(
                    instance_id,
                    ::std::any::type_name::<Self>(),
                );
                let result = Self {
                    __amethystate_instance_id: __amethystate_guard,
                    __amethystate_at: <Self as #crate_name::StateScope>::PATH.clone(),
                    #(#init_fields,)*
                };
                store.mark_initialized(&<Self as #crate_name::StateScope>::PATH)?;
                Ok(result)
            }
        }
    } else {
        quote! {
            /// Built by the struct that holds this one, and by nothing else.
            ///
            /// A struct with no `prefix` declares no place of its own: its
            /// fields sit under the field that holds it, which is where its
            /// path comes from. There is no door here that takes a path,
            /// because a declaration that could be put anywhere is one the
            /// schema layer cannot answer for - `Kv` would write over it and
            /// `Kv::clear` would take it away.
            ///
            /// What it is told is what its holder decided about a value it
            /// cannot read, a key removed under it, and an entry of a map that
            /// will not read. Whatever this struct declared for itself wins;
            /// these are what a field falls back to when neither said.
            ///
            /// Written by the macro, called by the macro.
            #[doc(hidden)]
            pub fn new_with_id_under(
                store: &#crate_name::Store,
                namespace: impl #crate_name::store::IntoStorePath,
                instance_id: #crate_name::uuid::Uuid,
                __ame_on_unreadable: #crate_name::store::OnUnreadable,
                __ame_on_delete: #crate_name::store::OnDelete,
                __ame_unreadable_entries: #crate_name::store::UnreadableEntries,
            ) -> ::core::result::Result<Self, #crate_name::store::OpenStruct> {
                use #crate_name::{StoreBackend, StoreExt};
                let namespace = namespace.into_store_path()?;
                let __amethystate_guard = #crate_name::store::instances::InstanceGuard::new(
                    instance_id,
                    ::std::any::type_name::<Self>(),
                );
                let result = Self {
                    __amethystate_instance_id: __amethystate_guard,
                    __amethystate_at: namespace.clone(),
                    #(#init_fields,)*
                };
                store.mark_initialized(&namespace)?;
                Ok(result)
            }
        }
    }
}
