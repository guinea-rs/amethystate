mod common;
use amethystate::StoreBuilder;
use amethystate::migration::ComponentOutcome;
use amethystate::store::builder::Backend;
use serde_json::json;
use std::path::Path;
#[cfg(feature = "s01")]
pub mod v01;
#[cfg(all(feature = "s01", not(feature = "s02")))]
use v01 as current;
#[cfg(feature = "s02")]
pub mod v02;
#[cfg(all(feature = "s02", not(feature = "s03")))]
use v02 as current;
#[cfg(feature = "s03")]
pub mod v03;
#[cfg(all(feature = "s03", not(feature = "s04")))]
use v03 as current;
#[cfg(feature = "s04")]
pub mod v04;
#[cfg(all(feature = "s04", not(feature = "s05")))]
use v04 as current;
#[cfg(feature = "s05")]
pub mod v05;
#[cfg(all(feature = "s05", not(feature = "s06")))]
use v05 as current;
#[cfg(feature = "s06")]
pub mod v06;
#[cfg(all(feature = "s06", not(feature = "s07")))]
use v06 as current;
#[cfg(feature = "s07")]
pub mod v07;
#[cfg(all(feature = "s07", not(feature = "s08")))]
use v07 as current;
#[cfg(feature = "s08")]
pub mod v08;
#[cfg(all(feature = "s08", not(feature = "s09")))]
use v08 as current;
#[cfg(feature = "s09")]
pub mod v09;
#[cfg(all(feature = "s09", not(feature = "s10")))]
use v09 as current;
#[cfg(feature = "s10")]
pub mod v10;
#[cfg(all(feature = "s10", not(feature = "s11")))]
use v10 as current;
#[cfg(feature = "s11")]
pub mod v11;
#[cfg(all(feature = "s11", not(feature = "s12")))]
use v11 as current;
#[cfg(feature = "s12")]
pub mod v12;
#[cfg(all(feature = "s12", not(feature = "s13")))]
use v12 as current;
#[cfg(feature = "s13")]
pub mod v13;
#[cfg(all(feature = "s13", not(feature = "s14")))]
use v13 as current;
#[cfg(feature = "s14")]
pub mod v14;
#[cfg(all(feature = "s14", not(feature = "s15")))]
use v14 as current;
#[cfg(feature = "s15")]
pub mod v15;
#[cfg(all(feature = "s15", not(feature = "s16")))]
use v15 as current;
#[cfg(feature = "s16")]
pub mod v16;
#[cfg(all(feature = "s16", not(feature = "s17")))]
use v16 as current;
#[cfg(feature = "s17")]
pub mod v17;
#[cfg(all(feature = "s17", not(feature = "s18")))]
use v17 as current;
#[cfg(feature = "s18")]
pub mod v18;
#[cfg(all(feature = "s18", not(feature = "s19")))]
use v18 as current;
#[cfg(feature = "s19")]
pub mod v19;
#[cfg(all(feature = "s19", not(feature = "s20")))]
use v19 as current;
#[cfg(feature = "s20")]
pub mod v20;
#[cfg(all(feature = "s20", not(feature = "s21")))]
use v20 as current;
#[cfg(feature = "s21")]
pub mod v21;
#[cfg(all(feature = "s21", not(feature = "s22")))]
use v21 as current;
#[cfg(feature = "s22")]
pub mod v22;
#[cfg(all(feature = "s22", not(feature = "s23")))]
use v22 as current;
#[cfg(feature = "s23")]
pub mod v23;
#[cfg(all(feature = "s23", not(feature = "s24")))]
use v23 as current;
#[cfg(feature = "s24")]
pub mod v24;
#[cfg(all(feature = "s24", not(feature = "s25")))]
use v24 as current;
#[cfg(feature = "s25")]
pub mod v25;
#[cfg(feature = "s25")]
use v25 as current;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 5);
    let path = Path::new(&args[1]);
    let snapshot = Path::new(&args[2]);
    let new_schema: bool = args[3].parse().unwrap();
    let expected_schema: u32 = args[4].parse().unwrap();
    let declared: Vec<_> = amethystate::schema::declarations()
        .iter()
        .filter(|entry| entry.prefix.to_string() == "halloy")
        .map(|entry| entry.version)
        .collect();
    assert_eq!(declared.iter().max(), Some(&expected_schema));
    let applied;
    {
        let (store, report) = StoreBuilder::new(path)
            .backend(Backend::Json)
            .migrate()
            .unwrap();
        assert!(!report.has_failures(), "{report:?}");
        assert!(!report.has_drift(), "{report:?}");
        applied = report
            .components
            .iter()
            .flat_map(|component| match &component.outcome {
                ComponentOutcome::Committed { steps } => steps.as_slice(),
                _ => &[],
            })
            .map(|step| step.target_version)
            .collect::<Vec<_>>();
        assert_eq!(
            applied,
            if new_schema && expected_schema > 1 {
                vec![expected_schema]
            } else {
                vec![]
            }
        );
        current::exercise(&store, new_schema);
        if expected_schema == 1 && new_schema {
            store.set(["outside", "note"], &"not ours").unwrap();
        }
        assert_eq!(
            store.get::<String>(["outside", "note"]).unwrap().as_deref(),
            Some("not ours")
        );
        store.save_now().unwrap();
    }
    {
        let (store, report) = StoreBuilder::new(path)
            .backend(Backend::Json)
            .migrate()
            .unwrap();
        assert!(!report.has_failures(), "{report:?}");
        assert!(!report.has_drift(), "{report:?}");
        assert!(
            report
                .components
                .iter()
                .all(|component| match &component.outcome {
                    ComponentOutcome::Committed { steps } => steps.is_empty(),
                    _ => true,
                })
        );
        current::check(&store);
        store.save_now().unwrap();
    }
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    std::fs::write(
        snapshot,
        serde_json::to_vec_pretty(
            &json!({"data": data, "applied": applied, "schema": expected_schema}),
        )
        .unwrap(),
    )
    .unwrap();
}
