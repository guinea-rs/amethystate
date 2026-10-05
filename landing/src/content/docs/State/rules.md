---
title: Rules
sidebar:
  order: 4
---

A window position of -32000, a font size of zero and the name of a theme nobody
installed all decode perfectly.
[When a value will not read](/amethystate/state/unreadable-values/) is about
bytes that will not read; a `rule` is where the application says what the ones
that do should be.

A rule is declared once, on the field, and it stands at every way a value goes
through that field, in both directions:

- **in** - read from the store as the struct is built or loaded, brought in by
  an edit to the file, written past the field by path with `Store::set`;
- **out** - written by this process with `set`, `update` or `modify`, or saved
  with a loaded struct's `save`.

The declared default is not among them. It is the declaration's own value and
is taken as written: where the store holds nothing, after `UseDefault`, and as
a `volatile` field's first value. A default the rule would change is a default
to change.

A rule does not refuse. It takes the value by `&mut` and leaves it acceptable:
a size out of range is clamped, a theme that is not installed is replaced.
Whatever it leaves is what the field holds and what the store holds - a
correction made on the way in is written back to the store, and one made by
`save` is left in the struct in hand. So no caller has to trim a value before
`set` to keep it in range, and no screen shows a value the file does not hold.

## Writing one

A rule is a bare `fn` taking the value and a context, and returning nothing.

<!-- shown: a rule on a field, and the world it is judged against -->
```rust
fn a_size_that_renders(size: &mut u8, _cx: &RuleContext) {
    *size = (*size).clamp(6, 72);
}

fn a_theme_that_is_installed(theme: &mut String, cx: &RuleContext) {
    let installed = cx.require::<InstalledThemes>();

    if !installed.0.contains(&theme.as_str()) {
        *theme = "dark".to_string();
    }
}

#[amethystate(prefix = "ruled_ui")]
pub struct Ui {
    #[amestate(default = 14u8, rule = a_size_that_renders)]
    pub font_size: u8,

    #[amestate(default = "dark".to_string(), rule = a_theme_that_is_installed)]
    pub theme: String,
}
```
<!-- /shown -->

The context is the answer to a rule being a bare `fn`: it captures nothing, so
the world it judges a value against - which monitors exist, which themes are
installed - is handed to the store when it opens.

```rust
let store = StoreBuilder::new(settings)
    .context(InstalledThemes(installed))
    .build()?;
```

One value per type, asked for with `cx.get::<T>()`, which answers `None` when
nothing was given, or `cx.require::<T>()`, which panics then. A store built
without what its rules need is built wrong the same way on every run, and the
panic names the type asked for and lists what was given instead.

## A write it puts right

A write lands as the rule left it, in the field and in the store alike.

<!-- shown: a write the rule puts right -->
```rust
ui.font_size().set(200);

assert_eq!(ui.font_size().get(), 72);
assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(72));
```
<!-- /shown -->

## Where a rule runs, and where it does not

| a value | a field's rule |
| --- | --- |
| read as the struct is built | runs; a correction is written back |
| written through the field: `set`, `update`, `modify` | runs; the write lands as the rule left it |
| written past the field: `Store::set` by path | runs as it arrives; a correction is written back |
| an edit from outside the process | runs as it arrives, the same way |
| `load_with` | runs; a correction is written back |
| `save` on a loaded struct | runs on every field before any is written |
| a migration step | does not run |

**A write past the field is put right after it was stored.** The store took
it before the field heard of it, so for a moment the file holds what arrived.
The correction is then written back over it, and the file ends up holding the
value the field holds.

**A correction the store will not take is a disagreement.** A rule can turn a
value into one the engine cannot hold - an infinity, for an engine that writes
JSON. The field then holds what the rule made of it, the store what it was
given, and `try_get` answers `Reason::NotWrittenBack` until a change lands.

`save` takes the struct by `&mut` and puts every field through its rule before
anything is written. What a rule puts right is put right in the struct in hand
as well, so after a save it holds what the file holds.

## Where a rule goes, and where it does not

A `volatile` field takes a rule too. Nothing arrives at it from the store, but
every write goes through it, so the rule is what keeps a value this process
holds and never stores in range.

Two fields will not take a rule at all, and the macro says so while it
compiles:

- **A `nested` field.** A field's rule judges one value at one path. A nested
  struct is not a value but a subtree of paths, so there is nothing to call a
  rule with. The rules go on the nested struct's own fields.
- **A map.** A rule is declared once against one value, and a map's entries
  are data - they come and go while the program runs, and there is no declared
  path to hang a rule on. What a map does with an entry it cannot read is
  `unreadable_entries`, in [Fields](/amethystate/state/fields/), not this
  page's subject.

Nor does a struct take one. A struct is the place its fields are kept rather
than a value of its own, so an invariant between two fields - a smallest width
no wider than the largest - is put right where the struct is opened:
[Opening a struct your own way](/amethystate/state/opening-structs/).
