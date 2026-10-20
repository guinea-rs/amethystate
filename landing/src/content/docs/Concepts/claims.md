---
title: Path ownership
sidebar:
  order: 16
---

A struct's `prefix` and each field's path together determine its storage
location. Different declarations can therefore refer to the same value:

<!-- shown: two structs that want the same place -->
```rust
#[amethystate(prefix = "ui", version = 1)]
pub struct Ui {
    #[amestate(path = "panels.left.visible", default = true)]
    pub left_panel_visible: bool,
}

#[amethystate(prefix = "ui.panels", version = 1)]
pub struct Panels {
    #[amestate(path = "left.visible", default = true)]
    pub left_visible: bool,
}
```
<!-- /shown -->

Both declarations address `ui.panels.left.visible`, even though they split
the prefix and field path differently.

## The second one is refused

The store records claimed paths. Opening a struct with a conflicting claim
returns an error:

<!-- shown: what the refusal looks like -->
```rust
let _ui = Ui::new_with(&store);

let refused = Panels::try_new_with(&store)
    .expect_err("`ui.panels.left.visible` is spelled by both of them");

let OpenStruct::Taken(taken) = &refused else {
    panic!("{refused}")
};

let Taken {
    at,
    wanted_by,
    held_at,
    held_by,
} = &**taken;

println!("{wanted_by} wants {at}, which {held_by} already holds at {held_at}");
```
<!-- /shown -->

`OpenStruct::Taken` identifies both declarations: `at` and `wanted_by` describe
the requested path and struct; `held_at` and `held_by` describe the existing
claim. The paths can differ when one is an ancestor of the other.

Match the error's fields rather than parsing its message. See
[Errors](/amethystate/concepts/errors/).

The check prevents independent state structs from silently overwriting each
other's data.

## What counts as overlapping

Paths conflict if they are equal or one is an ancestor of the other. The rule
is independent of declaration order and compares segments, not string prefixes.

| the two places | overlap |
| --- | --- |
| `ui` and `ui.panels` | yes: an ancestor and descendant |
| `ui.panels` and `ui` | yes: the same conflict in reverse |
| `ui.panels` and `ui.status` | no |
| `ui` and `ui!x` | no: different segment names |

A struct cannot be opened beneath another struct's leaf field. For example,
a `root.b` prefix conflicts with an existing field stored at `root.b`.

Sibling paths are allowed:

<!-- shown: a struct that sits right beside one and still opens -->
```rust
#[amethystate(prefix = "ui.panels.right", version = 1)]
pub struct RightPanel {
    #[amestate(default = true)]
    pub visible: bool,
}
```
<!-- /shown -->

`Ui` uses `ui.panels.left.visible`; `RightPanel` uses
`ui.panels.right.visible`. Neither owns an ancestor of the other, so both can
open. A declaration claiming `ui.panels` would conflict with both.

## What each kind of place owns

- **A leaf reserves its path and descendants.** Another declaration cannot
  store children beneath a value treated as one encoded leaf.
- **A map reserves its path and subtree.** Its entries occupy the next level;
  reserving the subtree protects keys that may be created later.
- **A struct does not reserve its prefix alone.** Its fields reserve their
  paths, leaving unrelated sibling paths available.

For example, this declaration:

<!-- shown: a map and a leaf under one prefix -->
```rust
#[amethystate(prefix = "ui")]
pub struct Panels {
    #[amestate(default = {})]
    pub open: ReactiveMap<String, u32>,

    #[amestate(default = "dark".to_string())]
    pub theme: String,
}
```
<!-- /shown -->

leaves these paths available or reserved:

```
ui                       ·  nobody's - a prefix is not claimed
├─ theme                 ▪  Panels declared it; nothing may go under it
├─ open                  ▪  Panels declared it, as a map
│  ├─ left               ▪    an entry, named while the program ran
│  └─ right              ▪    an entry
└─ myplugin              ·  nobody's - an extension may write here
```

`ui.myplugin` is available because the struct has not claimed all of `ui`.
`ui.open.myplugin` is reserved beneath `ui.open`:

```
under a struct's prefix            under a map's prefix
  ui                                 ui.open
  ├─ theme       declared            ├─ left       an entry
  ├─ open        declared            ├─ right      an entry
  └─ myplugin    allowed             └─ myplugin   refused
```

Reserving a subtree does not make every descendant a valid map entry.
`ui.open.left` is an entry, but `ui.open.left.px` is not: a map's entries occupy
exactly one level beneath it.

**Value nesting is separate from path depth.** A complex value stored in an
entry is encoded at that one path; its internal struct members do not become
independent store paths:

```
tree.roots.one           ▪  one key, one stored value
                            { name: "a", children: [ { … }, { … } ] }
                            the depth in here belongs to the codec
```

## The claim outlives the handle

Dropping a state struct does not release its paths. Claims belong to the store
and remain until that store is closed. This makes collision checks independent
of local variable lifetimes.

## Asking who has a place

<!-- shown: asking who claimed a place -->
```rust
let field = StorePath::parse_joined("ui.panels.left.visible")?;
let owner = store.places().declared_by(&field);

println!("{owner:?}");
```
<!-- /shown -->

The lookup requires the exact claimed path, not the struct's prefix.
The example `Ui` is registered at `ui.panels.left.visible`, not `ui`.
The result is the schema name also used in collision errors.
