---
title: Rules
sidebar:
  order: 4
---

A window position of -32000, a font size of zero and the name of a theme nobody
installed all decode perfectly.
[When a value will not read](/amethystate/state/unreadable-values/) is about
bytes that will not read; a `rule` is where the application says what it will
not have among the ones that do.

A rule is declared once, on the field, and it stands at every way a value goes
through that field, in both directions:

- **in** - read from the store as the struct is built or loaded, brought in by
  an edit to the file, written past the field by path with `Store::set`;
- **out** - written by this process with `set`, `update` or `modify`, or saved
  with a loaded struct's `save`.

The declared default is not among them. It is the declaration's own value and
is taken as written: where the store holds nothing, after `UseDefault`, and as
a `volatile` field's first value. A default the rule would refuse is a default
to change.

Whatever it decides reaches both sides. A value it puts right is what the field
holds and what the store holds: a correction made on the way in is written back
to the store, and one made by `save` is left in the struct in hand. A value it
refuses on the way out is written nowhere. So no caller has to trim a value
before `set` to keep it in range, and no screen shows a value the file does not
hold.

## Writing one

A rule is a bare `fn` taking the value and a context, and it answers with a
reason rather than a `bool` - the reason is what `try_get` reports, what a
refused open carries and what a refused write says, so it is written for
whoever has to fix the value.

It takes the value by `&mut`, so a rule that knows what the value should have
been may put it right and answer `Ok` instead. Nothing says a repair happened -
a value that passes is a value that passes, and there is no third state between
*accepted* and *refused*.

<!-- shown: a rule on a field, and the world it is judged against -->
```rust
fn a_size_that_renders(size: &mut u8, _cx: &RuleContext) -> Result<(), Invalid> {
    if *size >= 6 {
        Ok(())
    } else {
        Err(Invalid::new("a font size below 6 renders nothing"))
    }
}

fn a_theme_that_is_installed(theme: &mut String, cx: &RuleContext) -> Result<(), Invalid> {
    let installed = cx.require::<InstalledThemes>()?;

    if installed.0.contains(&theme.as_str()) {
        Ok(())
    } else {
        Err(Invalid::new(format!(
            "no theme called {theme} is installed"
        )))
    }
}

#[amethystate(prefix = "checked_lenient", on_unreadable = UseDefault)]
pub struct LenientUi {
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

One value per type, asked for with `cx.get::<T>()` or `cx.require::<T>()`.
`require` refuses the value when nothing was given, because a rule that cannot
reach its world cannot say the value is good.

## A write it turns down

A write the rule refuses does not land: the field keeps what it held, nothing is
written, and the write answers `WriteValue::Refused` with the path and the
reason.

<!-- shown: a write the rule turns down -->
```rust
match ui.font_size().set(3) {
    Ok(()) => {}
    Err(WriteValue::Refused { at, said }) => eprintln!("{at} will not take it: {said}"),
    Err(other) => return Err(other.into()),
}
```
<!-- /shown -->

A write it puts right lands as the rule left it, in the field and in the store
alike.

## A stored value it turns down

A refused value read from the store is the situation `on_unreadable` already
describes, and it is answered the same way: `Refuse` fails construction naming
the path and the reason, `UseDefault` takes the declared default, leaves the
stored value on disk and answers `try_get` with `Err` until a change passes.

A construction that fails hands back an `OpenStruct` - the six ways a
constructor can fail and no others - so a caller tells one refusal from another
by its variant rather than by reading the sentence:

<!-- shown: telling one refused open from another -->
```rust
match StrictUi::new_with(&store) {
    Ok(_) => {}
    Err(OpenStruct::Refused { at, said }) => eprintln!("{at} will not do: {said}"),
    Err(OpenStruct::WillNotRead { at, why }) => eprintln!("{at} is unreadable: {why}"),
    Err(OpenStruct::Taken(taken)) => {
        eprintln!("{} already holds {}", taken.held_by, taken.at)
    }
    Err(other) => return Err(other.into()),
}
```
<!-- /shown -->

Under `UseDefault` nothing fails, and the same place and reason arrive through
the field instead - as a `Disagreement`, which is what the field and the store
do not agree about rather than a failure of the asking:

<!-- shown: asking a field what the store disagrees with -->
```rust
let held = match ui.font_size().try_get() {
    Ok(size) => size,
    Err(no) => {
        match no.reason {
            Reason::Refused(said) => eprintln!("running on the default: {said}"),
            Reason::WillNotRead(said) => eprintln!("{} will not decode: {said}", no.at),
            Reason::Occupied(said) => eprintln!("{} was already taken: {said}", no.at),
            _ => eprintln!("{} is not what the store has", no.at),
        }

        ui.font_size().get()
    }
};
```
<!-- /shown -->

`Reason::Refused` is there only when a declared rule turned the value down;
bytes that would not decode arrive as `WillNotRead` with the codec's own
sentence, and a field that never got to write its default because the store
already held something arrives as `Occupied`. A field whose store has closed
answers `Reason::Closed`, which is none of those three and says the value is the
last one it heard. And `Reason::NotWrittenBack` is a rule that put a stored
value right while the store would not take the correction: the field holds what
the rule made of it, the store what it was given.

`Reason` is `#[non_exhaustive]` - it is a list of what a field can be found
disagreeing about, and it grows - so a `match` over it keeps a `_` arm. The
sets do not, and [Errors](/amethystate/concepts/errors/) is why.

## Where a rule runs, and where it does not

| a value | a field's rule |
| --- | --- |
| read as the struct is built | runs; a correction is written back |
| written through the field: `set`, `update`, `modify` | runs; a refusal leaves the field and the store as they were |
| written past the field: `Store::set` by path | runs as it arrives; a refusal keeps the last good value and wakes nobody, a correction is written back |
| an edit from outside the process | runs as it arrives, the same way |
| `load_with` | runs; a correction is written back |
| `save` on a loaded struct | runs on every field before any is written; a refusal fails the save |
| a migration step | does not run |

Two of those rows are worth reading twice.

**A write past the field is judged after it was stored.** The store took it
before the field heard of it, so what the rule refuses stays in the file, the
field keeps its last good value, and the write answers that a subscriber could
not take it. What the rule puts right is written back over what arrived, so
the file ends up holding the value the field holds.

**Under `mode = "persistent"` there is no `Field`, so there is no `try_get`.**
A refused value under `UseDefault` takes the declared default and says so in the
log, and that is the only place it is said. `Refuse` - the default - fails the
load instead, which is the answer to reach for when a loaded struct has to be
trustworthy.

`save` takes the struct by `&mut` and puts every field through its rule before
anything is written. What a rule puts right is put right in the struct in hand
as well, so after a save it holds what the file holds. A refusal fails the save
with `WriteValue::Refused`, and since every rule has run before the first
write, none of the struct is written.

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
no wider than the largest - is checked where the struct is opened:
[Opening a struct your own way](/amethystate/state/opening-structs/).

A runtime [interceptor](/amethystate/concepts/subscriptions/#interceptors) is there for
what a declaration cannot say: a rule that comes and goes while the
program runs, installed on a handle and taken off again.
