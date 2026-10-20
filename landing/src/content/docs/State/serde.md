---
title: Serde and storage layouts
sidebar:
  order: 6
---

State declarations and stored values use different attributes.

A struct marked `#[amethystate]`, including a `nested` component, describes
storage paths. Its fields are stored separately; the declaration itself is
not serialized as one value. Use `#[amestate(..)]` to control those paths and
field behavior.

A serde helper attribute on the declaration or its fields does not configure
amethystate. Without a corresponding serde derive, rustc rejects it:

```
error: cannot find attribute `serde` in this scope
  = note: `serde` is an attribute that can be used by the derive macros
          `Deserialize` and `Serialize`, you might be missing a `derive` attribute
```

The macro forwards attributes it does not handle to the generated fields;
it does not translate serde attributes into storage declarations.

A **leaf value** is different: it is serialized as one value at one path.
Its type can derive serde traits and use serde attributes normally.

## What says it instead

For state declarations, use these attributes:

| in serde | here |
| --- | --- |
| `rename = ".."` | `#[amestate(path = "..")]`; dots separate path segments rather than forming one literal name |
| `rename_all = ".."` | `rename_all` on `#[amethystate(..)]` |
| `default` on a field | `#[amestate(default = ..)]`, with an expression rather than a function path |
| `flatten` | `#[amestate(flatten)]` on a `nested` field; changes its storage paths |
| `with`, `serialize_with`, `deserialize_with` | the corresponding `#[amestate(..)]` attributes, using the same functions |

Attributes such as `tag`, `untagged`, `transparent` and `deny_unknown_fields`
configure a serialized value, not a collection of independently stored fields.
For unreadable fields use `on_unreadable`; for fields that must stay in memory
use `volatile`. To move previously saved data, write a migration step.
`#[rename(old => new)]` records a rename but does not replace the conversion
function's body.

See [Fields](/amethystate/state/fields/) for the complete attribute reference.

## Inside a leaf

The codec serializes the entire leaf value. Its serde attributes therefore
apply normally:

<!-- shown: a leaf, where serde answers to nobody here -->
```rust
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Endpoint {
    pub host_name: String,

    #[serde(default)]
    pub port: u16,
}

#[amethystate(prefix = "svc")]
pub struct Service {
    #[amestate(default = Endpoint { host_name: "localhost".to_string(), port: 443 })]
    pub upstream: Endpoint,
}
```
<!-- /shown -->

Here `Endpoint` is stored at `svc.upstream`; serde controls its member names
and rejects unknown fields during deserialization.

Changes inside a leaf need separate attention:

- **Internal field renames are not detected as schema drift.** The stored path
  stays the same. Ensure the new type can read old data, or convert it in a
  migration.
- **`deny_unknown_fields` restricts forward compatibility.** An older type
  rejects members added by a newer release.
- **New fields need a missing-value policy.** In formats with named members,
  `#[serde(default)]` lets a new type read data written before that field existed.
  Without a default, a missing required member fails the whole value.
- **Positional layouts need particular care.** Inserting a tuple member can
  shift subsequent values. Named Rust fields alone do not guarantee a named
  storage layout; check the selected codec.

## What a format will carry is a different list

Serde support does not guarantee support in every engine. A codec may reject
an enum, a non-finite float or an out-of-range integer even when the type
implements serde traits. See [Choosing an engine](/amethystate/choosing/)
for tested format limits.
