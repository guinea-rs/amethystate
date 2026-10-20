---
title: Rules
sidebar:
  order: 4
---

A value can deserialize successfully and still be unsuitable for the
application: a font size of zero, for example, or the name of an unavailable
theme. Use a `rule` to normalize such values. Data that cannot deserialize at
all is handled by [read policies](/amethystate/state/unreadable-values/).

A field's rule runs on values read from storage and written by the application:

- **in** - read from the store as the struct is built or loaded, brought in by
  an edit to the file, written past the field by path with `Store::set`;
- **out** - written by this process with `set`, `update` or `modify`, or saved
  with a loaded struct's `save`.

**Defaults do not pass through the rule.** This includes a missing key,
`UseDefault` and a volatile field's initial value. Choose a default that already
satisfies the rule.

A rule modifies a value through `&mut`; it does not reject it. It can clamp a
size or replace a missing theme. The field uses the corrected value, and a
correction made while reading is written back to the store. When saving a
loaded struct, the corrected value also remains in that struct. Callers do not
need to repeat the same normalization before each write.

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

A rule is a function pointer, not a capturing closure. Pass dependencies such
as the available monitors or installed themes through the store's context:

```rust
let store = StoreBuilder::new(settings)
    .context(InstalledThemes(installed))
    .build()?;
```

The context stores one value per type. `cx.get::<T>()` returns `None` when the
type was not supplied. `cx.require::<T>()` panics with the missing and available
types, making a missing required dependency visible during setup.

## A write it puts right

The field and store receive the rule's normalized value:

<!-- shown: a write the rule puts right -->
```rust
ui.font_size().set(200);

assert_eq!(ui.font_size().get(), 72);
assert_eq!(store.get::<u8>(["ruled_ui", "font_size"])?, Some(72));
```
<!-- /shown -->

## Where a rule runs, and where it does not

| operation | behavior |
| --- | --- |
| Opening or loading a field | Normalize the read value and write corrections back. |
| `set`, `update`, `modify` through the field | Normalize before writing. |
| `Store::set` by path | Normalize when the live field receives the change, then write corrections back. |
| External file edit | Process like an incoming stored value. |
| Saving a persistent-only struct | Run every field rule before starting writes. |
| Migration step | Do not run field rules. |

A direct path write is accepted by the store before the live field handles it.
The field then normalizes it and writes a correction if needed. Do not treat
this as a store-wide validation gate before the original write.

If writing a correction fails, the field retains its normalized value and
`try_get` reports `Reason::NotWrittenBack`.

Persistent-only `save` takes `&mut self`, so normalized values remain in the
loaded struct as well as being written to storage.

## Where a rule goes, and where it does not

Volatile fields can have rules for local writes, even though nothing is persisted.
The macro rejects `rule` on:

- **Nested fields:** they describe several paths, not one leaf value. Put rules
  on the component's individual fields.
- **Maps:** dynamic entries do not have individual field declarations to carry
  this rule. `unreadable_entries` controls read failures, not normalization.

Struct-wide rules are not supported. Put cross-field construction checks in
[Open](/amethystate/state/opening-structs/), and maintain the invariant in your
application's later updates.
