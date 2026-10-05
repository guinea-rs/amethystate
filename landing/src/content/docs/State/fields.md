---
title: Fields
sidebar:
  order: 2
---

Field attributes are optional. A field with no `#[amestate]` annotation uses `Default::default()` as its value and the field name as its storage key.

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    pub counter: u32, // no annotation — uses Default::default(), stored as "app.counter"

    #[amestate(default = 8080)]
    pub port: u16,
}
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `path` | `&str` | Where the field is stored, when that is not its own name. A dot in it is a level, so a field can be put anywhere under the prefix and not only renamed. |
| `default` | `Expr` | Initial value on first run. If omitted, uses `Default::default()`. |
| `nested` | flag | Marks field as an embedded `#[amethystate]` struct. |
| `flatten` | flag | On a `nested` field: its fields sit at this level, with no segment named after it. |
| `volatile` | flag | In-memory only. Never read from or written to the store. Resets to default on every restart. |
| `with`, `serialize_with`, `deserialize_with` | path | The functions this field is written and read through, when its own type is not what writes it. |
| `on_unreadable` | variant | This field's answer, overriding the struct's. A map has no default to stand in for one entry, so writing it there is a compile error naming `unreadable_entries` instead. See [When a value will not read](/amethystate/state/unreadable-values/). |
| `on_delete` | variant | The same for a deleted key. On a map it is the level this is about and not the entries - removing an entry is a removal under either answer. When the level itself goes, `UseDefault` puts the declared entries back and `Keep` (the default) leaves the map empty. |
| `unreadable_entries` | variant | On a map, and only on a map: `Skip` leaves out an entry it cannot read and builds the rest, `Refuse` (the default) fails and names the entry. |
| `rule` | `fn` | A rule every value from the store or from a write has to pass, on every way in and out: read, brought in by an edit, or written; the declared default does not go through it. What it puts right reaches the field and the store alike. See [Rules](/amethystate/state/rules/). |

They can be written in one `#[amestate(..)]` or spread over several, whichever
reads better:

```rust
#[amestate(path = "panels.left.visible")]
#[amestate(default = true, on_delete = Keep)]
pub left_panel_visible: bool,
```

Saying one of them twice is a compile error naming it: the second would win and
the first would look like it had been read.

## Where a field goes

`path` names the place a field is stored at, and `rename_all` on the struct says
it once for all of them. A dot in a `path` is a level, so a field can be put
anywhere under the prefix and not only renamed:

<!-- shown: a struct that says where its fields go -->
```rust
#[amethystate(prefix = "net", rename_all = "camelCase")]
pub struct NetState {
    #[amestate(default = 8080u16)]
    pub listen_port: u16,

    #[amestate(path = "tls.enabled", default = false)]
    pub tls: bool,
}
```
<!-- /shown -->

That writes `net.listenPort` and `net.tls.enabled`.

## A field whose paths sit at its holder's level

`flatten` on a `nested` field says its fields are stored here, without a segment
named after it:

<!-- shown: a nested struct whose fields sit at their holder's level -->
```rust
#[amethystate(prefix = "editor")]
pub struct Editor {
    #[amestate(nested, flatten)]
    pub window: Window,
}
```
<!-- /shown -->

That writes `editor.width`, not `editor.window.width`.

Two flattened children that reach one place are a compile error naming both,
since each stores its fields at this level and the two would write over each
other. So is a flattened child that reaches the place of a field written beside
it. Places are compared a level at a time, and a field's place is its path and
everything under it: `path = "window.width"` beside a flattened child that holds
a `window` struct is refused too, though neither spells the other's name whole.

**Both `path` and `flatten` decide where data lands, so changing either on
something already shipped is a migration.** The data stays where the old build
wrote it while the new build looks somewhere else, and what a person sees is
their settings gone back to defaults.

## A field stored some other way

When a type's own encoding is not the one you want on disk, the field is written
and read through a pair of functions of your own:

```rust
#[amestate(with = since_the_epoch)]
pub opened: SystemTime,
```

`with = m` is `m::serialize`, which writes, and `m::deserialize`, which reads.
Either half can be named on its own, as `serialize_with` or `deserialize_with`.
Then the type does the other half, and the value goes to disk one way and comes
back another. That is usually a mistake, so write both unless you want exactly
that difference.

Nothing else touches the value on its way to disk. What lies at the path is what
the first function wrote, and only the second turns it back.

The type itself still has to implement `Serialize` and `Deserialize`. Every
field's type does, and `with` chooses the form on disk rather than standing in
for them. A type from another crate that has neither goes into a newtype of your
own, which implements both - through the same pair of functions, if that is the
form you want.

The macro checks what it is given and names what it accepts, so a misspelling
is a compile error rather than an attribute that does nothing.

## Attributes that are not this macro's

Everything else written on a field is carried onto the field the macro
generates, and onto its getter. A doc comment arrives where it was aimed;
`#[allow]` and `#[deprecated]` do what they say; and an attribute nobody here
understands is judged by whoever does — which is how `#[serde(..)]` becomes
rustc's own error about an attribute that is not in scope, rather than something
this macro has an opinion about.

`#[cfg]` is the exception, and it is refused. A field appears in a dozen places
in what is generated — the struct, its constructor, the snapshot, the schema
written to disk — and some of those are `const` arrays, where an element cannot
be conditional. Carried to the places that allow it and not the rest, a field
compiled out would be missing from the struct and present in the schema, and
nothing would say so. Put the whole struct behind the `cfg`, or keep the field
and decide at runtime what it holds.

Which of serde's words do not carry over here, and why:
[What serde says here](/amethystate/state/serde/).

## Volatile fields

Volatile fields live in memory only and reset to their default on every restart. Useful for transient UI state that should not persist.

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    #[amestate(default = 8080)]
    pub port: u16,

    #[amestate(default = false, volatile)]
    pub loading: bool, // always starts as false, never written to disk
}
```

A volatile field still takes a [rule](/amethystate/state/rules/): every write
goes through it, so it keeps in range a value this process holds and never
stores.
