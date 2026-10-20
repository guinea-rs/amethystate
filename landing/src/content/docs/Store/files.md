---
title: What is on disk
sidebar:
  order: 15
---

Use the store's file description to find its data, metadata and recovery
files. Their names depend on the engine; do not reconstruct them yourself:

<!-- shown: asking a store where its files are -->
```rust
match StoreBackend::files_layout(&store) {
    Some(StoreLayout::Single { data }) => {
        println!("everything is in {}", data.display());
    }
    Some(StoreLayout::Sidecars {
        data,
        meta,
        data_backup,
        meta_backup,
    }) => {
        println!("values:      {}", data.display());
        println!("bookkeeping: {}", meta.display());
        println!(
            "kept while rewriting: {}, {}",
            data_backup.display(),
            meta_backup.display(),
        );
    }
    Some(StoreLayout::InMemory) => println!("nothing is on disk"),
    Some(_) => println!("a layout this code does not know yet"),
    None => println!("this engine does not say"),
}
```
<!-- /shown -->

The returned enum describes the engine's file layout. Match its variant to
access a particular file. Paths are reported even when the corresponding
file, such as a temporary backup, does not currently exist.

## redb and SQLite: one file

Both engines keep values and schema metadata in their database. They do not
use a separate amethystate `.meta` file.

## The text engines: two files, and two more while writing

| file | purpose |
| --- | --- |
| `settings.json` | stored values |
| `settings.json.meta` | schema versions, field descriptions and migration state |
| `settings.json.bak` | recovery copy of data during an opening rewrite |
| `settings.json.meta.bak` | recovery copy of metadata during that rewrite |

The data document can be read independently, but the store needs its metadata
to retain schema history. Include both files in your own backups.

Suffixes are appended to the **complete filename**. This keeps
`settings.user.meta` separate from `settings.system.meta` and avoids collisions
between the data and metadata backups.

Before release 0.23, metadata used names such as `settings.meta`. If the new
`settings.json.meta` is absent, opening reads the legacy file and writes the
new name. The legacy file is left in place because another store may use it.

## What the backup is for

A `.bak` is temporary protection during opening, not a user backup. The store
first reads the files and runs migrations in memory. Immediately before
rewriting a changed file, it creates a recovery copy. Successful completion
removes that copy. Unchanged files are neither copied nor rewritten.

A leftover `.bak` can therefore indicate an interrupted open. This example
constructs such a state:

<!-- shown: a previous open that never finished -->
```rust
std::fs::write(&backup, &good).unwrap();
std::fs::write(&data, "{ this never finished").unwrap();
```
<!-- /shown -->

The data file is incomplete, but its backup is readable. On the next open,
the store recovers from the backup and logs a warning under the `amethystate`
target. It first preserves the unreadable file as `settings.json.unreadable`.
After the recovered file is written successfully, the backup is removed.

Recovery copies are made **after a successful read**, immediately before a
rewrite. Copying unreadable input over a valid backup would destroy the only
recoverable data. A migration failure does not create new recovery copies or
rewrite the original files.

Unchanged files are left alone so that opening one store does not replace
another store's recent writes with a stale copy.

## Starting over

Deleting the data file starts a fresh store only when no data `.bak` remains.
In that case, leftover `.meta` does not restore the old state: the store uses
current schema versions, and state structs initialize their defaults.

## What the copy cannot promise

Recovery is based on whether the data file parses, not whether it is complete
or recent:

- A truncated document may still be syntactically valid. The store cannot
  distinguish missing keys from keys that were never written.
- If the document parses, it is accepted and the temporary recovery copy can
  be removed without restoring it.
- The backup has no trusted generation marker. A readable copy can be older
  than expected; its modification time is not a reliable version identifier.

Treat a leftover `.bak` as a recovery artifact, not a restore point. Keep
application backups separately when data loss would matter.

SQLite and redb use their database commit protocols instead of this text-file
recovery mechanism: SQLite uses a write-ahead log, while redb commits new pages
with a two-phase header update. These avoid exposing half-committed batches.
