/// The TypeScript a Rust type arrives as over JSON: the name the type is
/// written under, and the whole type.
pub fn map_type_to_ts(ty: syn::Type) -> (String, String) {
    match ty {
        syn::Type::Path(type_path) => match type_path.path.segments.last() {
            Some(segment) => path_to_ts(segment),
            None => any(),
        },
        syn::Type::Tuple(tuple) if tuple.elems.is_empty() => same("null"),
        syn::Type::Tuple(tuple) => {
            let items: Vec<String> = tuple
                .elems
                .iter()
                .map(|item| map_type_to_ts(item.clone()).1)
                .collect();
            let whole = format!("[{}]", items.join(", "));
            (whole.clone(), whole)
        }
        syn::Type::Array(array) => list_of(&array.elem),
        syn::Type::Slice(slice) => list_of(&slice.elem),
        syn::Type::Reference(reference) => map_type_to_ts(*reference.elem),
        syn::Type::Paren(paren) => map_type_to_ts(*paren.elem),
        syn::Type::Group(group) => map_type_to_ts(*group.elem),
        _ => any(),
    }
}

fn path_to_ts(segment: &syn::PathSegment) -> (String, String) {
    let arguments: Vec<syn::Type> = match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };

    match (segment.ident.to_string().as_str(), arguments.as_slice()) {
        ("String" | "str" | "char" | "SmolStr" | "PathBuf", _) => same("string"),
        ("bool", _) => same("boolean"),
        (
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
            | "isize" | "f32" | "f64",
            _,
        ) => same("number"),
        ("Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "IndexSet", [item]) => list_of(item),
        ("Option", [inner]) => {
            let (base, whole) = map_type_to_ts(inner.clone());
            (base, format!("{whole} | null"))
        }
        ("HashMap" | "BTreeMap" | "IndexMap", [_, value]) => {
            let whole = format!("Record<string, {}>", map_type_to_ts(value.clone()).1);
            (whole.clone(), whole)
        }
        ("Box" | "Rc" | "Arc" | "Cow", [.., inner]) => map_type_to_ts(inner.clone()),
        (other, _) => same(other),
    }
}

fn list_of(item: &syn::Type) -> (String, String) {
    let (base, whole) = map_type_to_ts(item.clone());
    let element = match whole.contains(' ') {
        true => format!("({whole})"),
        false => whole,
    };
    (base, format!("{element}[]"))
}

fn same(name: &str) -> (String, String) {
    (name.to_string(), name.to_string())
}

fn any() -> (String, String) {
    same("any")
}
