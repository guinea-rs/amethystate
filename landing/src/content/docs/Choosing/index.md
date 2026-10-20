---
title: Choosing a storage engine
sidebar:
  label: Overview
  order: 21
---

Storage engines do not preserve every value in the same way. These comparisons
show which values are rejected, which distinctions are lost and what the files
contain after writing.

Some differences come from the format, others from the engine's codec and value
representation. For example, TOML has no null, and JSON object keys are strings.
Do not assume that serde support alone makes a type portable.
[Store limits](/amethystate/store/limits/) describes the checks the library can
apply before a write.

## Every page here is a run

The comparison pages are generated from tests in
`crates/main/amethystate/tests`. `cargo xtask docs` runs each probe and includes
its marked code and actual output. To change a finding, edit the probe's
explanation or assertions and regenerate the page; manual page edits are
replaced on the next run.

## What earns a page

A comparison documents a meaningful loss or rejection, not just a harmless
formatting difference. The tags at the top make related findings searchable
by engine and topic.

## Reading a page

Read the explanation together with the code and output. The code shows the
exact operation tested; the table or documents show its result. The explanation
still needs review: generation keeps the output current, but cannot prove
that the prose interprets it correctly.

When choosing an engine, check the values your application actually uses.
After choosing one, use its results to plan validation and migrations.
